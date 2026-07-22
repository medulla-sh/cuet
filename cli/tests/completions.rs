use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cuet-completion-test-{}", std::process::id()));
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "/infra/neon:");
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
    assert!(
        candidates
            .lines()
            .any(|candidate| candidate == "--workspace")
    );
}
