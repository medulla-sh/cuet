use crate::cli::{Env, MigrationCommand, validate_env};
use crate::environment;
use crate::execution::{
    capture_tf_in, export_historical_backend, export_terraform, export_terraform_to,
    export_terraform_with_backend_to, output_tf_in, read_backend_config, read_migration_metadata,
    read_terraform_config, replace_root_backend, resolve_tool, run_tf_in, run_tfmigrate,
};
use crate::logger::Logger;
use crate::workspace::Workspace;
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;

const DEFAULT_TFMIGRATE_BIN: &str = "tfmigrate";
const LOCAL_BACKEND_OVERRIDE_VALUE: &str = r#""local.tfstate""#;

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

pub struct MigrationRunner<'a> {
    logger: &'a Logger,
    workspace: &'a Workspace,
    env: &'a str,
    cue_bin: &'a std::path::Path,
    tf_bin: Option<&'a std::path::Path>,
    tfmigrate_path: Option<&'a std::path::Path>,
    timeout: Option<Duration>,
    backend_override_value: &'a str,
}

impl<'a> MigrationRunner<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        logger: &'a Logger,
        workspace: &'a Workspace,
        env: &'a str,
        cue_bin: &'a std::path::Path,
        tf_bin: Option<&'a std::path::Path>,
        tfmigrate_path: Option<&'a std::path::Path>,
        timeout: Option<Duration>,
        backend_override_value: &'a str,
    ) -> Self {
        Self {
            logger,
            workspace,
            env,
            cue_bin,
            tf_bin,
            tfmigrate_path,
            timeout,
            backend_override_value,
        }
    }

    fn tf_bin(&self) -> Result<&std::path::Path> {
        self.tf_bin
            .ok_or_else(|| miette::miette!("Migration command requires OpenTofu"))
    }

    fn module_planner(&self) -> Result<ModuleMigrationPlanner<'_>> {
        Ok(ModuleMigrationPlanner::new(
            self.logger,
            self.workspace,
            self.env,
            self.cue_bin,
            self.tf_bin()?,
            self.timeout,
        ))
    }

    pub fn run(
        &self,
        command: &MigrationCommand,
        output: &mut impl Write,
    ) -> Result<Option<ExitStatus>> {
        let metadata: MigrationMetadata = read_migration_metadata(
            self.workspace,
            self.env,
            self.cue_bin,
            self.backend_override_value,
        )?;
        let transitions = self.current_transitions(&metadata);
        if !metadata.module_history.is_empty() && !transitions.is_empty() {
            return Err(miette::miette!(
                "Module history and cross-state resource history cannot be migrated together"
            ));
        }

        if let Some(target) = metadata.module_history.last() {
            return self.run_module(command, target, output);
        }

        if matches!(command, MigrationCommand::Check) {
            if transitions.is_empty() {
                return Err(miette::miette!(
                    "No migration history exists for {}:{}",
                    self.workspace.module_name(),
                    self.env
                ));
            }
            return Ok(None);
        }
        if matches!(command, MigrationCommand::Inspect) {
            serde_json::to_writer_pretty(&mut *output, &metadata).into_diagnostic()?;
            writeln!(output).into_diagnostic()?;
            return Ok(None);
        }

        let (status, destination_dir) = self.export_destination()?;
        if !status.success() {
            return Ok(Some(status));
        }
        let prepared = match self.prepare_resources(&transitions, &destination_dir)? {
            Preparation::Ready(prepared) => prepared,
            Preparation::CommandFailed(status) => return Ok(Some(status)),
        };

        let migration = migration_document(
            &prepared.source_dir,
            prepared.from_skip_plan,
            &prepared.actions,
        );
        let tfmigrate_bin = resolve_tool(
            self.tfmigrate_path
                .unwrap_or_else(|| std::path::Path::new(DEFAULT_TFMIGRATE_BIN)),
        )?;
        let (operation, args) = command
            .tfmigrate_command_and_args()
            .ok_or_else(|| miette::miette!("Migration command requires no tfmigrate operation"))?;
        run_tfmigrate(
            self.logger,
            &tfmigrate_bin,
            self.tf_bin()?,
            operation,
            args,
            &destination_dir,
            &migration,
            self.timeout,
        )
        .map(Some)
    }

    fn current_transitions<'m>(
        &self,
        metadata: &'m MigrationMetadata,
    ) -> Vec<&'m ResourceTransition> {
        metadata
            .resource_transitions
            .iter()
            .filter(|transition| {
                transition.to.module == self.workspace.module_name()
                    && transition.to.env == self.env
            })
            .collect()
    }

    fn export_destination(&self) -> Result<(ExitStatus, PathBuf)> {
        export_terraform(
            self.logger,
            self.workspace,
            self.env,
            self.cue_bin,
            self.backend_override_value,
        )
    }

    fn check_module_layout(&self, source_module: &str, source_env: &str) -> Result<()> {
        let source_module_path = workspace_module_path(self.workspace.root(), source_module)?;
        let source_lock = source_module_path
            .join(".cuet")
            .join(source_env)
            .join(".terraform.lock.hcl");
        let destination_lock = self
            .workspace
            .target_dir()
            .join(".cuet")
            .join(self.env)
            .join(".terraform.lock.hcl");
        let output_dir = self.workspace.target_dir().join(".cuet").join(self.env);
        let scratch_dir = output_dir.with_file_name(format!("{}.migrate", self.env));
        ensure_module_migration_files(&source_lock, &destination_lock, &output_dir, &scratch_dir)?;

        let mut current = read_terraform_config(self.workspace, self.env, self.cue_bin)?;
        let historical =
            read_backend_config(self.workspace, source_module, source_env, self.cue_bin)?;
        let has_required_providers = has_required_providers(&current);
        if has_required_providers && !destination_lock.is_file() {
            return Err(miette::miette!(
                "Destination provider lock is missing at '{}'",
                destination_lock.display()
            ));
        }
        if has_required_providers
            && std::fs::metadata(&destination_lock)
                .into_diagnostic()?
                .len()
                == 0
        {
            return Err(miette::miette!(
                "Destination provider lock is empty at '{}'",
                destination_lock.display()
            ));
        }
        replace_root_backend(&mut current, historical)?;
        Ok(())
    }

    fn inspect_module<'m>(
        &'m self,
        source_module: &'m str,
        source_env: &'m str,
    ) -> Result<ModuleMigrationInspection<'m>> {
        let source_backend =
            read_backend_config(self.workspace, source_module, source_env, self.cue_bin)?;
        let destination_backend = read_backend_config(
            self.workspace,
            self.workspace.module_name(),
            self.env,
            self.cue_bin,
        )?;
        let (source_backend, source_backend_location_complete) = inspected_backend(source_backend)?;
        let (destination_backend, destination_backend_location_complete) =
            inspected_backend(destination_backend)?;
        Ok(ModuleMigrationInspection {
            kind: "module",
            source: MigrationEndpoint {
                module: source_module,
                environment: source_env,
                backend: source_backend,
                backend_location_complete: source_backend_location_complete,
                lock_file: module_lock_path(source_module, source_env),
            },
            destination: MigrationEndpoint {
                module: self.workspace.module_name(),
                environment: self.env,
                backend: destination_backend,
                backend_location_complete: destination_backend_location_complete,
                lock_file: module_lock_path(self.workspace.module_name(), self.env),
            },
        })
    }

    fn run_module(
        &self,
        command: &MigrationCommand,
        target: &str,
        output: &mut impl Write,
    ) -> Result<Option<ExitStatus>> {
        if self.backend_override_value != "null" {
            return Err(miette::miette!(
                "Module migrations cannot be combined with --use-local-backend"
            ));
        }
        let (source_module, source_env) = parse_history_target(target, self.env)?;
        if source_module == self.workspace.module_name() && source_env == self.env {
            return Err(miette::miette!(
                "The latest module history target is the current target"
            ));
        }
        self.check_module_layout(source_module, source_env)?;
        if matches!(command, MigrationCommand::Check) {
            return Ok(None);
        }
        if matches!(command, MigrationCommand::Inspect) {
            let inspection = self.inspect_module(source_module, source_env)?;
            serde_json::to_writer_pretty(&mut *output, &inspection).into_diagnostic()?;
            writeln!(output).into_diagnostic()?;
            return Ok(None);
        }
        if !command.args().is_empty() {
            return Err(miette::miette!(
                "Additional migration arguments are supported only for resource migrations"
            ));
        }
        let (status, destination_dir) = self.export_destination()?;
        if !status.success() {
            return Ok(Some(status));
        }
        let migration_dir = MigrationDirectory::new(
            destination_dir.with_file_name(format!("{}.migrate", self.env)),
        );

        let status =
            self.module_planner()?
                .plan(source_module, source_env, migration_dir.path())?;
        if !status.success() || matches!(command, MigrationCommand::Plan { .. }) {
            return Ok(Some(status));
        }

        ModuleMigrationApplier::new(self.module_planner()?)
            .apply(source_module, source_env, migration_dir.path())
            .map(Some)
    }

    fn prepare_resources(
        &self,
        transitions: &[&ResourceTransition],
        destination_dir: &std::path::Path,
    ) -> Result<Preparation> {
        let Some(first) = transitions.first() else {
            return Err(miette::miette!(
                "No cross-state history exists for {}:{}",
                self.workspace.module_name(),
                self.env
            ));
        };
        ensure_single_source(transitions)?;
        let source_workspace = Workspace::resolve_workspace_relative(
            self.workspace.root(),
            Some(self.workspace.root()),
            std::path::Path::new(&first.from.module),
        )?;
        let (status, source_dir, from_skip_plan) =
            self.export_resource_source(&source_workspace, first, destination_dir)?;
        if !status.success() {
            return Ok(Preparation::CommandFailed(status));
        }

        Ok(Preparation::Ready(PreparedMigration {
            source_dir,
            actions: resource_actions(transitions),
            from_skip_plan,
        }))
    }

    fn export_resource_source(
        &self,
        source_workspace: &Workspace,
        transition: &ResourceTransition,
        destination_dir: &std::path::Path,
    ) -> Result<(ExitStatus, PathBuf, bool)> {
        let populated =
            environment::populated(self.cue_bin, source_workspace, self.backend_override_value)?;
        if populated.contains(&transition.from.env) {
            let (status, source_dir) = export_terraform(
                self.logger,
                source_workspace,
                &transition.from.env,
                self.cue_bin,
                self.backend_override_value,
            )?;
            return Ok((status, source_dir, false));
        }

        let source_dir = destination_dir.join(".tfmigrate/from");
        let status = export_historical_backend(
            self.logger,
            source_workspace,
            &transition.from.module,
            &transition.from.env,
            self.cue_bin,
            self.backend_override_value,
            &source_dir,
        )?;
        Ok((status, source_dir, true))
    }
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

pub struct MigrationDirectory(PathBuf);

impl MigrationDirectory {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for MigrationDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub struct ModuleMigrationPlanner<'a> {
    logger: &'a Logger,
    workspace: &'a Workspace,
    env: &'a str,
    cue_bin: &'a std::path::Path,
    tf_bin: &'a std::path::Path,
    timeout: Option<Duration>,
}

impl<'a> ModuleMigrationPlanner<'a> {
    pub fn new(
        logger: &'a Logger,
        workspace: &'a Workspace,
        env: &'a str,
        cue_bin: &'a std::path::Path,
        tf_bin: &'a std::path::Path,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            logger,
            workspace,
            env,
            cue_bin,
            tf_bin,
            timeout,
        }
    }

    pub fn plan(
        &self,
        source_module: &str,
        source_env: &str,
        migration_dir: &std::path::Path,
    ) -> Result<ExitStatus> {
        let status = self.prepare_source(source_module, source_env, migration_dir)?;
        if !status.success() {
            return Ok(status);
        }
        if matches!(
            self.read_state_snapshot(migration_dir)?,
            StateSnapshot::Missing
        ) {
            return Err(miette::miette!(
                "Source state is missing; cannot validate the migration"
            ));
        }
        let status = export_terraform_to(
            self.logger,
            self.workspace,
            self.env,
            self.cue_bin,
            LOCAL_BACKEND_OVERRIDE_VALUE,
            migration_dir,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let status = run_tf_in(
            self.logger,
            self.tf_bin,
            migration_dir,
            &[
                "init",
                "-migrate-state",
                "-force-copy",
                "-input=false",
                "-lockfile=readonly",
                "-lock-timeout=5m",
            ],
            self.timeout,
        )?;
        if !status.success() {
            return Ok(status);
        }
        run_tf_in(
            self.logger,
            self.tf_bin,
            migration_dir,
            &["plan", "-detailed-exitcode", "-lock-timeout=5m"],
            self.timeout,
        )
    }

    pub fn prepare_source(
        &self,
        source_module: &str,
        source_env: &str,
        migration_dir: &std::path::Path,
    ) -> Result<ExitStatus> {
        if migration_dir.exists() {
            std::fs::remove_dir_all(migration_dir).into_diagnostic()?;
        }
        let status = export_terraform_with_backend_to(
            self.logger,
            self.workspace,
            self.env,
            source_module,
            source_env,
            self.cue_bin,
            migration_dir,
        )?;
        if !status.success() {
            return Ok(status);
        }
        self.copy_provider_lock(migration_dir)?;
        let status = run_tf_in(
            self.logger,
            self.tf_bin,
            migration_dir,
            &["init", "-input=false", "-lockfile=readonly"],
            self.timeout,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let workspaces = output_tf_in(
            self.logger,
            self.tf_bin,
            migration_dir,
            &["workspace", "list", "-no-color"],
            self.timeout,
        )?;
        if !workspaces.status.success() {
            return Ok(workspaces.status);
        }
        ensure_default_workspace(&workspaces.stdout)?;
        Ok(status)
    }

    pub fn read_state_snapshot(&self, directory: &std::path::Path) -> Result<StateSnapshot> {
        let output = capture_tf_in(
            self.logger,
            self.tf_bin,
            directory,
            &["state", "pull"],
            self.timeout,
        )?;
        if output.status.success() {
            return state_snapshot_metadata(&output.stdout).map(StateSnapshot::Present);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if state_snapshot_missing(&stderr) {
            return Ok(StateSnapshot::Missing);
        }
        Err(miette::miette!(
            "Failed to read state snapshot: {}",
            stderr.trim()
        ))
    }

    fn copy_provider_lock(&self, destination_dir: &std::path::Path) -> Result<()> {
        let source = self
            .workspace
            .target_dir()
            .join(".cuet")
            .join(self.env)
            .join(".terraform.lock.hcl");
        if source.is_file() {
            std::fs::copy(&source, destination_dir.join(".terraform.lock.hcl"))
                .into_diagnostic()
                .map_err(|error| {
                    miette::miette!(
                        "Failed to copy provider lock from '{}': {error}",
                        source.display()
                    )
                })?;
        }
        Ok(())
    }
}

pub struct ModuleMigrationApplier<'a> {
    planner: ModuleMigrationPlanner<'a>,
}

impl<'a> ModuleMigrationApplier<'a> {
    pub fn new(planner: ModuleMigrationPlanner<'a>) -> Self {
        Self { planner }
    }

    pub fn apply(
        &self,
        source_module: &str,
        source_env: &str,
        migration_dir: &std::path::Path,
    ) -> Result<ExitStatus> {
        let status = self
            .planner
            .prepare_source(source_module, source_env, migration_dir)?;
        if !status.success() {
            return Ok(status);
        }
        let StateSnapshot::Present(source_metadata) =
            self.planner.read_state_snapshot(migration_dir)?
        else {
            return Err(miette::miette!(
                "Source state is missing; cannot verify that the migration was previously applied"
            ));
        };

        let destination_dir = MigrationDirectory::new(
            migration_dir.with_file_name(format!("{}.destination", self.planner.env)),
        );
        let status = self.prepare_destination(destination_dir.path())?;
        if !status.success() {
            return Ok(status);
        }
        if matches!(
            destination_action(
                &source_metadata,
                &self.planner.read_state_snapshot(destination_dir.path())?
            )?,
            DestinationAction::Current
        ) {
            return run_tf_in(
                self.planner.logger,
                self.planner.tf_bin,
                destination_dir.path(),
                &["plan", "-detailed-exitcode", "-lock-timeout=5m"],
                self.planner.timeout,
            );
        }
        let status = export_terraform_to(
            self.planner.logger,
            self.planner.workspace,
            self.planner.env,
            self.planner.cue_bin,
            "null",
            migration_dir,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let status = run_tf_in(
            self.planner.logger,
            self.planner.tf_bin,
            migration_dir,
            &[
                "init",
                "-migrate-state",
                "-lockfile=readonly",
                "-lock-timeout=5m",
            ],
            self.planner.timeout,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let destination_state = output_tf_in(
            self.planner.logger,
            self.planner.tf_bin,
            migration_dir,
            &["state", "pull"],
            self.planner.timeout,
        )?;
        if !destination_state.status.success() {
            return Ok(destination_state.status);
        }
        let destination_metadata = state_snapshot_metadata(&destination_state.stdout)?;
        ensure_migrated_snapshot(&source_metadata, &destination_metadata)?;
        run_tf_in(
            self.planner.logger,
            self.planner.tf_bin,
            migration_dir,
            &["plan", "-detailed-exitcode", "-lock-timeout=5m"],
            self.planner.timeout,
        )
    }

    fn prepare_destination(&self, destination_dir: &std::path::Path) -> Result<ExitStatus> {
        if destination_dir.exists() {
            std::fs::remove_dir_all(destination_dir).into_diagnostic()?;
        }
        let status = export_terraform_to(
            self.planner.logger,
            self.planner.workspace,
            self.planner.env,
            self.planner.cue_bin,
            "null",
            destination_dir,
        )?;
        if !status.success() {
            return Ok(status);
        }
        self.planner.copy_provider_lock(destination_dir)?;
        run_tf_in(
            self.planner.logger,
            self.planner.tf_bin,
            destination_dir,
            &["init", "-input=false", "-lockfile=readonly"],
            self.planner.timeout,
        )
    }
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

struct PreparedMigration {
    source_dir: PathBuf,
    actions: Vec<String>,
    from_skip_plan: bool,
}

enum Preparation {
    Ready(PreparedMigration),
    CommandFailed(ExitStatus),
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

fn workspace_module_path(root: &std::path::Path, module: &str) -> Result<PathBuf> {
    let path = std::path::Path::new(module);
    if module.is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(miette::miette!(
            "Historical module must be a workspace-relative path: {module}"
        ));
    }
    Ok(root.join(path))
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

fn module_lock_path(module: &str, env: &str) -> String {
    std::path::Path::new(module)
        .join(".cuet")
        .join(env)
        .join(".terraform.lock.hcl")
        .to_string_lossy()
        .into_owned()
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

pub fn state_snapshot_metadata(output: &[u8]) -> Result<StateSnapshotMetadata> {
    serde_json::from_slice(output)
        .into_diagnostic()
        .map_err(|error| miette::miette!("Failed to read state snapshot metadata: {error}"))
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
