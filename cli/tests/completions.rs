use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_completions_command_generates_dynamic_registration() {
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
            .env_remove("COMPLETE")
            .args(["completions", shell])
            .output()
            .unwrap();

        assert_success(&output);
        let registration = String::from_utf8(output.stdout).unwrap();
        assert!(registration.contains("cuet"));
        assert!(registration.contains("COMPLETE"));
    }
}

#[test]
fn test_dynamic_completion_traverses_workspace_module_folders() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".cuetroot.cue"), "").unwrap();
    let module = temp.path().join("infra/neon");
    fs::create_dir_all(&module).unwrap();
    fs::write(module.join("cuet.cue"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .current_dir(temp.path())
        .env("COMPLETE", "bash")
        .env_remove("_CLAP_IFS")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "cuet", "-t", ""])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "/infra/");

    for shell in ["bash", "elvish", "fish", "powershell"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
            .current_dir(temp.path())
            .env("COMPLETE", shell)
            .env_remove("_CLAP_IFS")
            .env("_CLAP_COMPLETE_INDEX", "2")
            .args(["--", "cuet", "-t", "/infra/"])
            .output()
            .unwrap();

        assert_success(&output);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim_end(),
            "/infra/neon:",
            "unexpected {shell} completion"
        );
    }
}

#[test]
fn test_zsh_registration_uses_removable_module_suffix() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .env_remove("COMPLETE")
        .args(["completions", "zsh"])
        .output()
        .unwrap();

    assert_success(&output);
    let registration = String::from_utf8(output.stdout).unwrap();
    assert!(registration.starts_with("#compdef cuet\n"));
    assert!(registration.contains("function _cuet()"));
    assert!(registration.contains("if [[ $funcstack[1] == _cuet ]]"));
    assert!(registration.contains("_describe -V 'modules' branch_modules -S '/' -r '/: '"));
    assert!(registration.contains("_describe -V 'modules' modules -S ':' -r ': '"));
}

#[test]
fn test_zsh_dynamic_completion_marks_leaf_modules() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".cuetroot.cue"), "").unwrap();
    let module = temp.path().join("infra/neon");
    fs::create_dir_all(&module).unwrap();
    fs::write(module.join("cuet.cue"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .current_dir(temp.path())
        .env("COMPLETE", "zsh")
        .env_remove("_CLAP_IFS")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "cuet", "-t", "/infra/"])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "__cuet_target_module__\t/infra/neon"
    );
}

#[test]
fn test_zsh_dynamic_completion_marks_modules_with_descendants() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".cuetroot.cue"), "").unwrap();
    let module = temp.path().join("infra");
    let nested_module = module.join("neon");
    fs::create_dir_all(&nested_module).unwrap();
    fs::write(module.join("cuet.cue"), "").unwrap();
    fs::write(nested_module.join("cuet.cue"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .current_dir(temp.path())
        .env("COMPLETE", "zsh")
        .env_remove("_CLAP_IFS")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "cuet", "-t", ""])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "__cuet_target_branch_module__\t/infra"
    );
}

#[test]
fn test_dynamic_completion_suggests_root_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .env("COMPLETE", "bash")
        .env_remove("_CLAP_IFS")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .args(["--", "cuet", "--"])
        .output()
        .unwrap();

    assert_success(&output);
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
fn test_dynamic_completion_suggests_modules_check_drift() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .env("COMPLETE", "bash")
        .env_remove("_CLAP_IFS")
        .env("_CLAP_COMPLETE_INDEX", "3")
        .args(["--", "cuet", "modules", "check", "--"])
        .output()
        .unwrap();

    assert_success(&output);
    let candidates = String::from_utf8(output.stdout).unwrap();
    assert!(candidates.lines().any(|candidate| candidate == "--drift"));
}

#[test]
fn test_dynamic_completion_suggests_version_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_cuet"))
        .env("COMPLETE", "bash")
        .env_remove("_CLAP_IFS")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .args(["--", "cuet", "ver"])
        .output()
        .unwrap();

    assert_success(&output);
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "version");
}
