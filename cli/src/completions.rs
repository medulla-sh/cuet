use crate::cli::{Cli, ModuleTarget};
use crate::environment;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use clap::CommandFactory;
use clap_complete::CompleteEnv;
use clap_complete::Shell;
use clap_complete::engine::CompletionCandidate;
use miette::{IntoDiagnostic, Result};
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn complete() {
    CompleteEnv::with_factory(Cli::command).complete();
}

pub fn write_registration(shell: Shell, output: &mut impl Write) -> Result<()> {
    let executable = std::env::current_exe().into_diagnostic()?;
    let registration = Command::new(executable)
        .env("COMPLETE", shell.to_string())
        .output()
        .into_diagnostic()?;
    if !registration.status.success() {
        return Err(miette::miette!(
            "Failed to generate {shell} completions ({}): {}",
            registration.status,
            String::from_utf8_lossy(&registration.stderr).trim()
        ));
    }

    output.write_all(&registration.stdout).into_diagnostic()
}

pub fn target_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let Ok(current_dir) = std::env::current_dir() else {
        return Vec::new();
    };

    target_values(current, &current_dir, Path::new("cue"))
        .unwrap_or_default()
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

fn target_values(current: &str, current_dir: &Path, cue_bin: &Path) -> Result<Vec<String>> {
    let Some((module, environment_prefix)) = current.split_once(':') else {
        let root = resolve_root(current_dir, None)?;
        return Ok(discover_modules(&root)?
            .into_iter()
            .map(|module| {
                if module == "." {
                    ":".to_owned()
                } else {
                    format!("/{module}:")
                }
            })
            .filter(|candidate| module_candidate_matches(candidate, current))
            .collect());
    };

    let module_target = if module.is_empty() {
        ModuleTarget::Relative(PathBuf::from("."))
    } else if let Some(module) = module.strip_prefix('/') {
        ModuleTarget::WorkspaceRelative(PathBuf::from(module))
    } else {
        ModuleTarget::Relative(PathBuf::from(module))
    };
    let workspace = Workspace::resolve(current_dir, None, &module_target)?;
    let mut environments: Vec<_> = environment::populated(cue_bin, &workspace, "null")?
        .into_iter()
        .filter(|environment| environment.starts_with(environment_prefix))
        .map(|environment| format!("{module}:{environment}"))
        .collect();
    environments.sort();
    Ok(environments)
}

fn module_candidate_matches(candidate: &str, current: &str) -> bool {
    candidate.starts_with(current)
        || (!current.starts_with('/')
            && candidate
                .strip_prefix('/')
                .is_some_and(|candidate| candidate.starts_with(current)))
}

#[cfg(test)]
mod tests {
    use super::target_values;
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::ffi::OsStr;
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_target_values_complete_workspace_modules_without_cue() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("infra/neon")).into_diagnostic()?;
        fs::create_dir_all(root.join("services/api")).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(root.join("cuet.cue"), "").into_diagnostic()?;
        fs::write(root.join("infra/neon/cuet.cue"), "").into_diagnostic()?;
        fs::write(root.join("services/api/cuet.cue"), "").into_diagnostic()?;

        let values = target_values("", &root, &temp.path().join("missing-cue"))?;

        assert_eq!(values, [":", "/infra/neon:", "/services/api:"]);
        assert_eq!(
            target_values("infra/n", &root, &temp.path().join("missing-cue"))?,
            ["/infra/neon:"]
        );
        Ok(())
    }

    #[test]
    fn test_target_values_complete_selected_module_environments() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        let cue = temp.path().join("cue");
        let mut file = File::create(&cue).into_diagnostic()?;
        file.write_all(b"#!/usr/bin/env bash\nprintf '[\"prod\",\"dev\",\"stage\"]'\n")
            .into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        drop(file);
        fs::set_permissions(&cue, fs::Permissions::from_mode(0o755)).into_diagnostic()?;

        let values = target_values("/infra/neon:d", &root, &cue)?;

        assert_eq!(values, ["/infra/neon:dev"]);
        Ok(())
    }

    #[test]
    fn test_target_values_ignore_discovery_errors_at_completion_boundary() {
        let values = super::target_candidates(OsStr::new(""));

        assert!(values.is_empty());
    }
}
