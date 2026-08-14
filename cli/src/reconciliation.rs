use crate::cli::{Env, parse_env};
use crate::logger::Logger;
use crate::terraform::{INIT_STATE_FILE, capture_in, read_timeout};
use crate::workspace::Workspace;
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reconciliation<'a> {
    pub environment: &'a str,
    pub required_providers: Vec<HistoricalProvider>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct HistoricalProvider {
    pub source: String,
    pub alias: String,
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
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(environment) = parse_env(name) else {
            continue;
        };
        let path = entry.path();
        if !path.join(INIT_STATE_FILE).is_file() {
            continue;
        }
        environments.insert(environment);
    }
    Ok(environments)
}

pub fn inspect<'a>(
    logger: &Logger,
    workspace: &Workspace,
    tf_bin: &Path,
    environment: &'a str,
    timeout: Option<Duration>,
) -> Result<Option<Reconciliation<'a>>> {
    let directory = workspace.target_dir().join(".cuet").join(environment);
    if !directory.join(INIT_STATE_FILE).is_file() {
        return Ok(None);
    }
    let state = inspect_state(
        logger,
        tf_bin,
        &directory,
        environment,
        Some(read_timeout(timeout)),
    )?;
    if !state.has_state {
        return Ok(None);
    }
    Ok(Some(Reconciliation {
        environment,
        required_providers: state.providers.into_iter().collect(),
    }))
}

pub fn remove_if_empty(
    logger: &Logger,
    workspace: &Workspace,
    tf_bin: &Path,
    environment: &str,
    timeout: Option<Duration>,
) -> Result<()> {
    if inspect(logger, workspace, tf_bin, environment, timeout)?.is_some() {
        return Ok(());
    }
    remove_local(workspace, environment)
}

pub fn remove_local(workspace: &Workspace, environment: &str) -> Result<()> {
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
    providers: BTreeSet<HistoricalProvider>,
}

#[derive(Deserialize)]
struct PulledState {
    #[serde(rename = "version")]
    _version: u64,
    #[serde(default)]
    resources: Vec<StateResource>,
    #[serde(default)]
    outputs: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct StateResource {
    provider: String,
}

fn inspect_state(
    logger: &Logger,
    tf_bin: &Path,
    directory: &Path,
    environment: &str,
    timeout: Option<Duration>,
) -> Result<EnvironmentState> {
    let state = capture_in(logger, tf_bin, directory, &["state", "pull"], timeout)?;
    if !state.status.success() {
        let stderr = String::from_utf8_lossy(&state.stderr);
        if state_missing(&stderr) {
            return Ok(EnvironmentState {
                has_state: false,
                providers: BTreeSet::new(),
            });
        }
        return Err(miette::miette!(
            "Failed to inspect state for environment '{environment}' ({}): {}",
            state.status,
            stderr.trim()
        ));
    }

    let state: PulledState = serde_json::from_slice(&state.stdout)
        .into_diagnostic()
        .map_err(|error| {
            miette::miette!("OpenTofu returned invalid state for '{environment}': {error}")
        })?;
    let providers = state
        .resources
        .iter()
        .map(historical_provider)
        .collect::<Result<_>>()?;
    Ok(EnvironmentState {
        has_state: !state.resources.is_empty() || !state.outputs.is_empty(),
        providers,
    })
}

fn historical_provider(resource: &StateResource) -> Result<HistoricalProvider> {
    if resource.provider.starts_with("module.") {
        return Err(miette::miette!(
            "OpenTofu state contains an unsupported module-scoped provider configuration"
        ));
    }
    let address = resource
        .provider
        .strip_prefix("provider[\"")
        .ok_or_else(|| miette::miette!("OpenTofu returned invalid provider address"))?;
    let (source, suffix) = address
        .split_once("\"]")
        .ok_or_else(|| miette::miette!("OpenTofu returned invalid provider address"))?;
    let source = source
        .strip_prefix("registry.opentofu.org/")
        .or_else(|| source.strip_prefix("registry.terraform.io/"))
        .unwrap_or(source);
    let source_parts = source.split('/').collect::<Vec<_>>();
    if source != "terraform.io/builtin/terraform"
        && (!matches!(source_parts.len(), 2 | 3) || source_parts.iter().any(|part| part.is_empty()))
    {
        return Err(miette::miette!("OpenTofu returned invalid provider source"));
    }
    let alias = if suffix.is_empty() {
        ""
    } else {
        suffix
            .strip_prefix('.')
            .filter(|alias| !alias.is_empty() && !alias.contains(['.', '[', ']', '"']))
            .ok_or_else(|| miette::miette!("OpenTofu returned invalid provider alias"))?
    };
    Ok(HistoricalProvider {
        source: source.to_owned(),
        alias: alias.to_owned(),
    })
}

fn state_missing(stderr: &str) -> bool {
    stderr.contains("No state file was found")
}

#[cfg(test)]
mod tests {
    use super::{
        HistoricalProvider, StateResource, environment_names, historical_provider, inspect,
        remove_if_empty,
    };
    use crate::logger::Logger;
    use crate::test_directory::TestDirectory;
    use crate::workspace::Workspace;
    use miette::{IntoDiagnostic, Result};
    use std::fs;
    use std::path::PathBuf;

    fn initialize_environment(workspace: &Workspace, environment: &str) -> Result<PathBuf> {
        let directory = workspace.target_dir().join(".cuet").join(environment);
        let init_state = directory.join(".terraform/terraform.tfstate");
        fs::create_dir_all(init_state.parent().expect("state should have parent"))
            .into_diagnostic()?;
        fs::write(init_state, "").into_diagnostic()?;
        Ok(directory)
    }

    #[test]
    fn test_historical_provider_reads_source_and_alias() -> Result<()> {
        assert_eq!(
            historical_provider(&StateResource {
                provider: r#"provider["registry.opentofu.org/kislerdm/neon"]"#.to_owned(),
            })?,
            HistoricalProvider {
                source: "kislerdm/neon".to_owned(),
                alias: String::new(),
            }
        );
        assert_eq!(
            historical_provider(&StateResource {
                provider: r#"provider["registry.terraform.io/hashicorp/google-beta"].bootstrap"#
                    .to_owned(),
            })?,
            HistoricalProvider {
                source: "hashicorp/google-beta".to_owned(),
                alias: "bootstrap".to_owned(),
            }
        );
        assert_eq!(
            historical_provider(&StateResource {
                provider: r#"provider["providers.example.com/acme/cloud"]"#.to_owned(),
            })?,
            HistoricalProvider {
                source: "providers.example.com/acme/cloud".to_owned(),
                alias: String::new(),
            }
        );
        assert_eq!(
            historical_provider(&StateResource {
                provider: r#"provider["terraform.io/builtin/terraform"]"#.to_owned(),
            })?,
            HistoricalProvider {
                source: "terraform.io/builtin/terraform".to_owned(),
                alias: String::new(),
            }
        );
        Ok(())
    }

    #[test]
    fn test_historical_provider_rejects_invalid_source() {
        assert!(
            historical_provider(&StateResource {
                provider: r#"provider["example"]"#.to_owned(),
            })
            .is_err()
        );
    }

    #[test]
    fn test_historical_provider_rejects_unsupported_configurations() {
        for provider in [
            r#"module.database.provider["registry.opentofu.org/hashicorp/google"]"#,
            r#"provider["registry.opentofu.org/hashicorp/aws"].by_region["eu-west-1"]"#,
        ] {
            assert!(
                historical_provider(&StateResource {
                    provider: provider.to_owned(),
                })
                .is_err()
            );
        }
    }

    #[test]
    fn test_environment_names_lists_initialized_directories_without_inspecting_state() -> Result<()>
    {
        let temp = TestDirectory::new()?;
        let workspace = temp.workspace()?;
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
        let workspace = temp.workspace()?;
        let tf_bin = temp.path().join("tofu");
        temp.write_executable(
            &tf_bin,
            r#"#!/usr/bin/env bash
set -euo pipefail
environment="$(basename "$PWD")"
if [[ "$*" == "state pull" ]]; then
    case "$environment" in
        legacy) printf '%s' '{"version":4,"resources":[{"type":"neon_project","provider":"provider[\"registry.opentofu.org/kislerdm/neon\"]"},{"type":"google_secret_manager_secret_version","provider":"provider[\"registry.opentofu.org/hashicorp/google\"].bootstrap"}]}' ;;
        live) if [[ ! -f removed ]]; then printf '%s' '{"version":4,"resources":[{"type":"neon_project","provider":"provider[\"registry.opentofu.org/kislerdm/neon\"]"}]}'; else printf '{"version":4}'; fi ;;
        outputs) printf '%s' '{"version":4,"outputs":{"host":{"value":"example"}}}' ;;
        malformed) printf '{}' ;;
        *) printf '{"version":4}' ;;
    esac
fi
"#,
        )?;
        for environment in ["empty", "legacy", "live", "malformed", "outputs"] {
            initialize_environment(&workspace, environment)?;
        }

        let reconciliation = inspect(&Logger::new(false), &workspace, &tf_bin, "legacy", None)?
            .expect("legacy should have state");
        let output_reconciliation =
            inspect(&Logger::new(false), &workspace, &tf_bin, "outputs", None)?
                .expect("outputs should count as state");
        let empty = inspect(&Logger::new(false), &workspace, &tf_bin, "empty", None)?;
        let malformed =
            remove_if_empty(&Logger::new(false), &workspace, &tf_bin, "malformed", None);

        assert_eq!(reconciliation.environment, "legacy");
        assert_eq!(
            reconciliation.required_providers,
            [
                HistoricalProvider {
                    source: "hashicorp/google".to_owned(),
                    alias: "bootstrap".to_owned(),
                },
                HistoricalProvider {
                    source: "kislerdm/neon".to_owned(),
                    alias: String::new(),
                },
            ]
        );
        assert!(output_reconciliation.required_providers.is_empty());
        assert!(empty.is_none());
        assert!(malformed.is_err());
        assert!(workspace.target_dir().join(".cuet/empty").is_dir());
        assert!(workspace.target_dir().join(".cuet/malformed").is_dir());

        fs::write(workspace.target_dir().join(".cuet/live/removed"), "").into_diagnostic()?;
        remove_if_empty(&Logger::new(false), &workspace, &tf_bin, "live", None)?;

        assert!(!workspace.target_dir().join(".cuet/live").exists());
        Ok(())
    }
}
