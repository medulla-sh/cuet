use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("cuet-completion-test-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_completions_command_generates_dynamic_registration() {
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
            .args(["completions", shell])
            .output()
            .unwrap();

        assert!(output.status.success());
        let registration = String::from_utf8(output.stdout).unwrap();
        assert!(registration.contains("cuet"));
        assert!(registration.contains("COMPLETE"));
    }
}

#[test]
fn test_dynamic_completion_suggests_workspace_modules() {
    let temp = TestDirectory::new();
    fs::write(temp.path().join(".cuetroot.cue"), "").unwrap();
    let module = temp.path().join("infra/neon");
    fs::create_dir_all(&module).unwrap();
    fs::write(module.join("cuet.cue"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .current_dir(temp.path())
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "cuet", "-t", ""])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "/infra/neon");
}

#[test]
fn test_zsh_registration_uses_removable_module_suffix() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .args(["completions", "zsh"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let registration = String::from_utf8(output.stdout).unwrap();
    assert!(registration.contains("_describe -V 'modules' modules -S ':' -r ' '"));
}

#[test]
fn test_zsh_dynamic_completion_marks_workspace_modules() {
    let temp = TestDirectory::new();
    fs::write(temp.path().join(".cuetroot.cue"), "").unwrap();
    let module = temp.path().join("infra/neon");
    fs::create_dir_all(&module).unwrap();
    fs::write(module.join("cuet.cue"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .current_dir(temp.path())
        .env("COMPLETE", "zsh")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "cuet", "-t", ""])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "__cuet_target_module__\t/infra/neon"
    );
}

#[test]
fn test_dynamic_completion_suggests_root_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .args(["--", "cuet", "--"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let candidates = String::from_utf8(output.stdout).unwrap();
    assert!(candidates.lines().any(|candidate| candidate == "--target"));
    assert!(candidates.lines().any(|candidate| candidate == "--version"));
    assert!(
        candidates
            .lines()
            .any(|candidate| candidate == "--workspace")
    );
}

#[test]
fn test_dynamic_completion_suggests_version_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .args(["--", "cuet", "ver"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "version");
}
