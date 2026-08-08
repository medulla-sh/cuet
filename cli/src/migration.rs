use crate::cli::{Env, validate_env};
use miette::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationMetadata {
    pub module_history: Vec<String>,
    pub resource_transitions: Vec<ResourceTransition>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTransition {
    pub resource_type: String,
    pub from: ResourceIdentity,
    pub to: ResourceIdentity,
}

#[derive(Deserialize, Serialize)]
pub struct ResourceIdentity {
    pub module: String,
    pub env: Env,
    pub name: String,
}

#[derive(Deserialize)]
pub struct StateSnapshotMetadata {
    pub lineage: String,
    pub serial: u64,
    #[serde(flatten)]
    pub contents: BTreeMap<String, serde_json::Value>,
}

pub enum StateSnapshot {
    Missing,
    Present(StateSnapshotMetadata),
}

pub enum DestinationAction {
    Copy,
    Current,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationEndpoint<'a> {
    pub module: &'a str,
    pub environment: &'a str,
    pub backend: serde_json::Value,
    pub backend_location_complete: bool,
    pub lock_file: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleMigrationInspection<'a> {
    pub kind: &'static str,
    pub source: MigrationEndpoint<'a>,
    pub destination: MigrationEndpoint<'a>,
}

#[derive(Serialize)]
pub struct MigrationDocument<'a> {
    migration: MigrationBody<'a>,
}

#[derive(Serialize)]
struct MigrationBody<'a> {
    multi_state: MultiStateMigration<'a>,
}

#[derive(Serialize)]
struct MultiStateMigration<'a> {
    cuet: CuetMigration<'a>,
}

#[derive(Serialize)]
struct CuetMigration<'a> {
    from_dir: &'a std::path::Path,
    from_skip_plan: bool,
    to_dir: &'static str,
    actions: &'a [String],
}

pub fn ensure_default_workspace(output: &[u8]) -> Result<()> {
    let output = String::from_utf8_lossy(output);
    let workspaces: Vec<_> = output
        .lines()
        .map(|workspace| workspace.trim_start_matches('*').trim())
        .filter(|workspace| !workspace.is_empty())
        .collect();
    if workspaces != ["default"] {
        return Err(miette::miette!(
            "Module migration requires exactly the default workspace; found: {}",
            workspaces.join(", ")
        ));
    }
    Ok(())
}

pub fn ensure_module_migration_files(
    source_lock: &std::path::Path,
    destination_lock: &std::path::Path,
    output_dir: &std::path::Path,
    scratch_dir: &std::path::Path,
) -> Result<()> {
    if source_lock.exists() {
        return Err(miette::miette!(
            "Historical provider lock still exists at '{}'; move it to '{}'",
            source_lock.display(),
            destination_lock.display()
        ));
    }
    for artifact in [
        output_dir.join("tfmigrate.json"),
        output_dir.join("_tfmigrate_override.tf"),
        output_dir.join(".tfmigrate"),
    ] {
        if artifact.exists() {
            return Err(miette::miette!(
                "Stale tfmigrate artifact exists at '{}'; remove it for native module migration",
                artifact.display()
            ));
        }
    }
    if scratch_dir.exists() {
        return Err(miette::miette!(
            "Stale module migration directory exists at '{}'; remove it before continuing",
            scratch_dir.display()
        ));
    }
    Ok(())
}

fn root_backend_mut(config: &mut serde_json::Value) -> Result<&mut serde_json::Value> {
    config
        .get_mut("terraform")
        .and_then(|terraform| terraform.get_mut("backend"))
        .ok_or_else(|| miette::miette!("Terraform configuration has no root backend"))
}

pub fn inspected_backend(mut config: serde_json::Value) -> Result<(serde_json::Value, bool)> {
    const LOCATION_FIELDS: [&str; 14] = [
        "bucket",
        "container_name",
        "hostname",
        "key",
        "namespace",
        "organization",
        "path",
        "prefix",
        "project",
        "region",
        "resource_group_name",
        "secret_suffix",
        "storage_account_name",
        "workspace_key_prefix",
    ];
    let backend = root_backend_mut(&mut config)?
        .as_object_mut()
        .ok_or_else(|| miette::miette!("Terraform root backend must be an object"))?;
    if backend.len() != 1 {
        return Err(miette::miette!(
            "Terraform root backend must contain exactly one backend type"
        ));
    }
    let (backend_type, backend_config) = std::mem::take(backend)
        .into_iter()
        .next()
        .ok_or_else(|| miette::miette!("Terraform root backend is empty"))?;
    let serde_json::Value::Object(backend_config) = backend_config else {
        return Err(miette::miette!(
            "Terraform backend configuration must be an object"
        ));
    };
    let safe_config = backend_config
        .into_iter()
        .filter(|(key, _)| {
            LOCATION_FIELDS.contains(&key.as_str())
                || backend_type == "consul" && key == "address"
                || backend_type == "remote" && key == "workspaces"
        })
        .collect();
    let complete = matches!(backend_type.as_str(), "gcs" | "local");
    let mut backend = serde_json::Map::new();
    backend.insert(backend_type, serde_json::Value::Object(safe_config));
    Ok((serde_json::Value::Object(backend), complete))
}

pub fn has_required_providers(config: &serde_json::Value) -> bool {
    config
        .get("terraform")
        .and_then(|terraform| terraform.get("required_providers"))
        .and_then(serde_json::Value::as_object)
        .is_some_and(|providers| !providers.is_empty())
}

pub fn state_snapshot_missing(stderr: &str) -> bool {
    stderr.contains("No state file was found")
}

pub fn ensure_migrated_snapshot(
    source: &StateSnapshotMetadata,
    destination: &StateSnapshotMetadata,
) -> Result<()> {
    if source.lineage != destination.lineage || destination.serial < source.serial {
        return Err(miette::miette!(
            "The destination state does not contain the migrated source snapshot"
        ));
    }
    Ok(())
}

pub fn destination_action(
    source: &StateSnapshotMetadata,
    destination: &StateSnapshot,
) -> Result<DestinationAction> {
    let StateSnapshot::Present(destination) = destination else {
        return Ok(DestinationAction::Copy);
    };
    if destination.lineage != source.lineage {
        return Err(miette::miette!(
            "Destination state is non-empty and has unrelated lineage"
        ));
    }
    if destination.serial == source.serial && destination.contents == source.contents {
        return Ok(DestinationAction::Current);
    }
    Err(miette::miette!(
        "Destination state does not exactly match the source snapshot"
    ))
}

pub fn ensure_single_source(transitions: &[&ResourceTransition]) -> Result<()> {
    let sources: BTreeSet<_> = transitions
        .iter()
        .map(|transition| (&transition.from.module, &transition.from.env))
        .collect();
    if sources.len() != 1 {
        return Err(miette::miette!(
            "Pending resource migrations must originate from one module environment"
        ));
    }
    Ok(())
}

pub fn resource_actions(transitions: &[&ResourceTransition]) -> Vec<String> {
    let mut actions: Vec<_> = transitions
        .iter()
        .map(|transition| {
            shell_words::join([
                "mv",
                &format!("{}.{}", transition.resource_type, transition.from.name),
                &format!("{}.{}", transition.resource_type, transition.to.name),
            ])
        })
        .collect();
    actions.sort();
    actions
}

pub fn migration_document<'a>(
    source_dir: &'a std::path::Path,
    from_skip_plan: bool,
    actions: &'a [String],
) -> MigrationDocument<'a> {
    MigrationDocument {
        migration: MigrationBody {
            multi_state: MultiStateMigration {
                cuet: CuetMigration {
                    from_dir: source_dir,
                    from_skip_plan,
                    to_dir: ".",
                    actions,
                },
            },
        },
    }
}

pub fn parse_history_target<'a>(
    target: &'a str,
    default_env: &'a str,
) -> Result<(&'a str, &'a str)> {
    let (module, env) = target.split_once(':').unwrap_or((target, default_env));
    let module = module.strip_prefix('/').unwrap_or(module);
    if module.is_empty() {
        return Err(miette::miette!("History target module cannot be empty"));
    }
    validate_env(env).map_err(|error| miette::miette!(error))?;
    Ok((module, env))
}
