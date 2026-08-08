use miette::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

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
