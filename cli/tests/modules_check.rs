use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn workspace(module_count: usize) -> (TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(".cuetroot.cue"), "").unwrap();
    for index in 0..module_count {
        let module = root.join(format!("module-{index}"));
        fs::create_dir(&module).unwrap();
        fs::write(module.join("cuet.cue"), "").unwrap();
    }
    (temp, root)
}

fn write_cue(path: &Path) {
    write_executable(
        path,
        r#"#!/usr/bin/env bash
# Fake CUE discovers one environment and writes requested Terraform exports.
set -euo pipefail
expression=""
output=""
while [[ $# -gt 0 ]]; do
	case $1 in
	-e) expression=$2; shift 2 ;;
	-o) output=$2; shift 2 ;;
	*) shift ;;
	esac
done
if [[ $expression == *'["in"]'* ]]; then
	printf '["dev"]'
	exit 0
fi
if [[ -n $output ]]; then
	printf '{}' > "$output"
fi
"#,
    );
}

#[test]
fn test_modules_check_drift_runs_all_plans_concurrently_and_suppresses_output() {
    let (temp, root) = workspace(3);
    let cue = temp.path().join("cue");
    write_cue(&cue);
    let markers = temp.path().join("markers");
    fs::create_dir(&markers).unwrap();
    let tofu = temp.path().join("tofu");
    write_executable(
        &tofu,
        &format!(
            r#"#!/usr/bin/env bash
# Fake OpenTofu waits for every plan to prove they overlap.
set -euo pipefail
printf 'terraform stdout sentinel\n'
printf 'terraform stderr sentinel\n' >&2
if [[ $1 == init ]]; then
	exit 0
fi
module=$(basename "$(dirname "$(dirname "$PWD")")")
touch '{markers}/'"$module"
for ((attempt = 0; attempt < 200; attempt++)); do
	shopt -s nullglob
	files=('{markers}/'*)
	if (( ${{#files[@]}} == 3 )); then
		exit 0
	fi
	sleep 0.01
done
exit 12
"#,
            markers = markers.display()
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .args([
            "--workspace",
            root.to_str().unwrap(),
            "--cue-path",
            cue.to_str().unwrap(),
            "--tf-path",
            tofu.to_str().unwrap(),
            "modules",
            "check",
            "--drift",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("terraform stdout sentinel"));
    assert!(!stderr.contains("terraform stderr sentinel"));
    for index in 0..3 {
        assert!(stderr.contains(&format!("PASS Checking module-{index}:dev")));
    }
    assert!(!stderr.contains('\r'));
    assert!(!stderr.contains("\u{1b}["));
}

#[test]
fn test_modules_check_jobs_caps_concurrent_plans() {
    let (temp, root) = workspace(4);
    let cue = temp.path().join("cue");
    write_cue(&cue);
    let slots = temp.path().join("slots");
    fs::create_dir(&slots).unwrap();
    let overflow = temp.path().join("overflow");
    let tofu = temp.path().join("tofu");
    write_executable(
        &tofu,
        &format!(
            r#"#!/usr/bin/env bash
# Fake OpenTofu fails if more than two plans are active.
set -euo pipefail
if [[ $1 == init ]]; then
	exit 0
fi
if mkdir '{slots}/one' 2>/dev/null; then
	slot='{slots}/one'
elif mkdir '{slots}/two' 2>/dev/null; then
	slot='{slots}/two'
else
	touch '{overflow}'
	exit 13
fi
cleanup() {{
	rmdir "$slot"
}}
trap cleanup EXIT
sleep 0.05
"#,
            slots = slots.display(),
            overflow = overflow.display(),
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .args([
            "--workspace",
            root.to_str().unwrap(),
            "--cue-path",
            cue.to_str().unwrap(),
            "--tf-path",
            tofu.to_str().unwrap(),
            "modules",
            "check",
            "--drift",
            "--jobs",
            "2",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!overflow.exists());
}
