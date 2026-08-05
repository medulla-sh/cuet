use crate::cli::{Env, parse_env};
use crate::execution::{capture_tf_in, terraform_read_timeout};
use crate::logger::Logger;
use crate::workspace::Workspace;
use miette::{IntoDiagnostic, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

const TERRAFORM_INIT_STATE_FILE: &str = ".terraform/terraform.tfstate";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reconciliation {
    pub(crate) environment: Env,
    pub(crate) required_providers: Vec<String>,
}

pub fn environment_names(workspace: &Workspace) -> Result<BTreeSet<Env>> {
    let root = workspace.target_dir().join(".cuet");
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeSet::new());
        }
        Err(error) => {
            return Err(error).into_diagnostic().map_err(|error| {
                miette::miette!("Failed to inspect '{}': {error}", root.display())
            });
        }
    };
    let mut environments = BTreeSet::new();
    for entry in entries {
        let entry = entry.into_diagnostic()?;
        if !entry.file_type().into_diagnostic()?.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(environment) = parse_env(&name) else {
            continue;
        };
        if !entry.path().join(TERRAFORM_INIT_STATE_FILE).is_file() {
            continue;
        }
        environments.insert(environment);
    }
    Ok(environments)
}

pub fn inspect(
    logger: &Logger,
    workspace: &Workspace,
    tf_bin: &Path,
    environment: &Env,
    timeout: Option<Duration>,
) -> Result<Option<Reconciliation>> {
    let directory = workspace.target_dir().join(".cuet").join(environment);
    if !directory.join(TERRAFORM_INIT_STATE_FILE).is_file() {
        return Ok(None);
    }
    let state = inspect_state(
        logger,
        tf_bin,
        &directory,
        environment,
        Some(terraform_read_timeout(timeout)),
    )?;
    if !state.has_state {
        return Ok(None);
    }
    Ok(Some(Reconciliation {
        environment: environment.clone(),
        required_providers: state.providers.into_iter().collect(),
    }))
}

pub fn remove_if_empty(
    logger: &Logger,
    workspace: &Workspace,
    tf_bin: &Path,
    environment: &Env,
    timeout: Option<Duration>,
) -> Result<()> {
    if inspect(logger, workspace, tf_bin, environment, timeout)?.is_some() {
        return Ok(());
    }
    remove_local(workspace, environment)
}

pub fn remove_local(workspace: &Workspace, environment: &Env) -> Result<()> {
    let directory = workspace.target_dir().join(".cuet").join(environment);
    if !directory.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&directory)
        .into_diagnostic()
        .map_err(|error| {
            miette::miette!(
                "Failed to remove empty environment directory '{}': {error}",
                directory.display()
            )
        })
}

struct EnvironmentState {
    has_state: bool,
    providers: BTreeSet<String>,
}

fn inspect_state(
    logger: &Logger,
    tf_bin: &Path,
    directory: &Path,
    environment: &Env,
    timeout: Option<Duration>,
) -> Result<EnvironmentState> {
    let resources = capture_tf_in(logger, tf_bin, directory, &["state", "list"], timeout)?;
    if !resources.status.success() {
        let stderr = String::from_utf8_lossy(&resources.stderr);
        if state_missing(&stderr) {
            return Ok(EnvironmentState {
                has_state: false,
                providers: BTreeSet::new(),
            });
        }
        return Err(miette::miette!(
            "Failed to list state for environment '{environment}' ({}): {}",
            resources.status,
            stderr.trim()
        ));
    }

    let resource_addresses = String::from_utf8(resources.stdout)
        .into_diagnostic()
        .map_err(|error| {
            miette::miette!(
                "OpenTofu returned invalid state addresses for '{environment}': {error}"
            )
        })?;
    let providers = resource_addresses
        .lines()
        .filter(|address| !address.trim().is_empty())
        .map(provider_name)
        .collect::<Result<BTreeSet<_>>>()?;
    if !providers.is_empty() {
        return Ok(EnvironmentState {
            has_state: true,
            providers,
        });
    }

    let outputs = capture_tf_in(logger, tf_bin, directory, &["output", "-json"], timeout)?;
    if !outputs.status.success() {
        let stderr = String::from_utf8_lossy(&outputs.stderr);
        if state_missing(&stderr) {
            return Ok(EnvironmentState {
                has_state: false,
                providers,
            });
        }
        return Err(miette::miette!(
            "Failed to inspect outputs for environment '{environment}' ({}): {}",
            outputs.status,
            stderr.trim()
        ));
    }
    let outputs: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&outputs.stdout)
            .into_diagnostic()
            .map_err(|error| {
                miette::miette!(
                    "OpenTofu returned invalid output data for environment '{environment}': {error}"
                )
            })?;
    Ok(EnvironmentState {
        has_state: !outputs.is_empty(),
        providers,
    })
}

fn provider_name(address: &str) -> Result<String> {
    let segments = address_segments(address)?;
    let mut index = 0;
    while segments.get(index) == Some(&"module") {
        index += 2;
    }
    if segments.get(index) == Some(&"data") {
        index += 1;
    }
    let resource_type = segments
        .get(index)
        .ok_or_else(|| miette::miette!("OpenTofu returned invalid resource address '{address}'"))?;
    let (provider, _) = resource_type.split_once('_').ok_or_else(|| {
        miette::miette!(
            "Cannot infer a provider from resource type '{resource_type}' in state address '{address}'"
        )
    })?;
    Ok(provider.to_owned())
}

fn address_segments(address: &str) -> Result<Vec<&str>> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut bracket_depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in address.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' if bracket_depth > 0 => quoted = true,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '.' if bracket_depth == 0 => {
                segments.push(&address[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if quoted || bracket_depth != 0 {
        return Err(miette::miette!(
            "OpenTofu returned invalid resource address '{address}'"
        ));
    }
    segments.push(&address[start..]);
    Ok(segments)
}

fn state_missing(stderr: &str) -> bool {
    stderr.contains("No state file was found")
}

#[cfg(test)]
mod tests {
    use super::{environment_names, inspect, provider_name, remove_if_empty};
    use crate::cli::ModuleTarget;
    use crate::logger::Logger;
    use crate::test_support::TestDirectory;
    use crate::workspace::Workspace;
    use miette::{IntoDiagnostic, Result};
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    fn test_workspace(temp: &TestDirectory) -> Result<Workspace> {
        let root = temp.path().join("workspace");
        let module = root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        Workspace::resolve(&module, None, &ModuleTarget::Relative(PathBuf::from(".")))
    }

    fn write_executable(path: &Path, body: &str) -> Result<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(body.as_bytes()).into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        drop(file);
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).into_diagnostic()
    }

    fn initialize_environment(workspace: &Workspace, environment: &str) -> Result<PathBuf> {
        let directory = workspace.target_dir().join(".cuet").join(environment);
        let init_state = directory.join(".terraform/terraform.tfstate");
        fs::create_dir_all(init_state.parent().expect("state should have parent"))
            .into_diagnostic()?;
        fs::write(init_state, "").into_diagnostic()?;
        Ok(directory)
    }

    #[test]
    fn test_provider_name_reads_root_resource() -> Result<()> {
        assert_eq!(provider_name("neon_project.example")?, "neon");
        assert_eq!(
            provider_name("data.google_secret_manager_secret_version.neon")?,
            "google"
        );
        Ok(())
    }

    #[test]
    fn test_provider_name_reads_indexed_module_resource() -> Result<()> {
        assert_eq!(
            provider_name(r#"module.regions["us.west"].module.database.neon_project.example[0]"#)?,
            "neon"
        );
        Ok(())
    }

    #[test]
    fn test_provider_name_rejects_resource_without_prefix() {
        let error = provider_name("example.resource")
            .expect_err("resource type without provider prefix should fail");

        assert!(error.to_string().contains("Cannot infer a provider"));
    }

    #[test]
    fn test_environment_names_lists_initialized_directories_without_inspecting_state() -> Result<()>
    {
        let temp = TestDirectory::new()?;
        let workspace = test_workspace(&temp)?;
        for environment in ["dev", "legacy"] {
            initialize_environment(&workspace, environment)?;
        }
        fs::create_dir_all(workspace.target_dir().join(".cuet/not:valid")).into_diagnostic()?;

        let environments = environment_names(&workspace)?;

        assert_eq!(
            environments.iter().map(String::as_str).collect::<Vec<_>>(),
            ["dev", "legacy"]
        );
        Ok(())
    }

    #[test]
    fn test_inspect_reconciles_only_selected_environment() -> Result<()> {
        let temp = TestDirectory::new()?;
        let workspace = test_workspace(&temp)?;
        let tf_bin = temp.path().join("tofu");
        write_executable(
            &tf_bin,
            r#"#!/usr/bin/env bash
set -euo pipefail
environment="$(basename "$PWD")"
if [[ "$*" == "state list" ]]; then
    case "$environment" in
        legacy) printf '%s\n' 'neon_project.example' 'data.google_secret_manager_secret_version.neon' ;;
        live) if [[ ! -f removed ]]; then printf '%s\n' 'neon_project.example'; fi ;;
    esac
elif [[ "$*" == "output -json" ]]; then
    if [[ "$environment" == "outputs" ]]; then
        printf '{"host":{"value":"example"}}'
    else
        printf '{}'
    fi
fi
"#,
        )?;
        for environment in ["empty", "legacy", "live", "outputs"] {
            initialize_environment(&workspace, environment)?;
        }

        let reconciliation = inspect(
            &Logger::new(false),
            &workspace,
            &tf_bin,
            &"legacy".to_owned(),
            None,
        )?
        .expect("legacy should have state");
        let output_reconciliation = inspect(
            &Logger::new(false),
            &workspace,
            &tf_bin,
            &"outputs".to_owned(),
            None,
        )?
        .expect("outputs should count as state");
        let empty = inspect(
            &Logger::new(false),
            &workspace,
            &tf_bin,
            &"empty".to_owned(),
            None,
        )?;

        assert_eq!(reconciliation.environment, "legacy");
        assert_eq!(reconciliation.required_providers, ["google", "neon"]);
        assert!(output_reconciliation.required_providers.is_empty());
        assert!(empty.is_none());
        assert!(workspace.target_dir().join(".cuet/empty").is_dir());

        fs::write(workspace.target_dir().join(".cuet/live/removed"), "").into_diagnostic()?;
        remove_if_empty(
            &Logger::new(false),
            &workspace,
            &tf_bin,
            &"live".to_owned(),
            None,
        )?;

        assert!(!workspace.target_dir().join(".cuet/live").exists());
        Ok(())
    }
}
