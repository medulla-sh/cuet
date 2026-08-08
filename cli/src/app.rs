use crate::cli::{
    Cli, Commands, CueCommand, Env, MigrationCommand, ModuleTarget, ModulesCommand, parse_env,
};
use crate::completions;
use crate::environment;
use crate::execution::{
    TerraformMetadata, capture_tf_in, check_cue_export, export_historical_backend,
    export_terraform, export_terraform_to, export_terraform_with_backend_to, output_tf_in,
    read_backend_config, read_migration_metadata, read_terraform_config, replace_root_backend,
    resolve_tool, run_cue, run_tf_in, run_tf_with_metadata, run_tfmigrate,
};
use crate::logger::Logger;
use crate::reconciliation;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use clap::CommandFactory;
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::Component;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::thread;
use std::time::Duration;

const DEFAULT_CUE_BIN: &str = "cue";
const DEFAULT_TF_BIN: &str = "tofu";
const DEFAULT_TFMIGRATE_BIN: &str = "tfmigrate";
const LOCAL_BACKEND_OVERRIDE_VALUE: &str = r#""local.tfstate""#;

#[derive(Clone, Copy, Debug)]
enum TargetCommand<'a> {
    Cue(&'a CueCommand),
    Tf(&'a [String]),
    Migrate(&'a MigrationCommand),
}

pub fn run(cli: &Cli) -> Result<Option<ExitStatus>> {
    let current_dir = std::env::current_dir().into_diagnostic()?;
    run_from(cli, &current_dir, &mut io::stdout().lock())
}

fn run_from(
    cli: &Cli,
    current_dir: &std::path::Path,
    output: &mut impl Write,
) -> Result<Option<ExitStatus>> {
    match &cli.command {
        Commands::Version => {
            write!(output, "{}", Cli::command().render_version()).into_diagnostic()?;
            Ok(None)
        }
        Commands::Completions { shell } => {
            completions::write_registration(*shell, output)?;
            Ok(None)
        }
        Commands::Modules { command } => {
            let root = resolve_root(current_dir, cli.workspace.as_deref())?;
            run_modules(
                command,
                &root,
                cli.cue_path.as_deref(),
                cli.use_local_backend,
                cli.verbose,
                output,
            )?;
            Ok(None)
        }
        Commands::Cue { command } => {
            run_target_command(cli, TargetCommand::Cue(command), current_dir)
        }
        Commands::Tf { args } => run_target_command(cli, TargetCommand::Tf(args), current_dir),
        Commands::Migrate { command } => {
            run_target_command(cli, TargetCommand::Migrate(command), current_dir)
        }
    }
}

fn run_target_command(
    cli: &Cli,
    command: TargetCommand<'_>,
    current_dir: &std::path::Path,
) -> Result<Option<ExitStatus>> {
    let cue_bin = resolve_tool(
        cli.cue_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new(DEFAULT_CUE_BIN)),
    )?;
    let workspace = if let Some(target) = &cli.target {
        Workspace::resolve(current_dir, cli.workspace.as_deref(), &target.module)?
    } else {
        Workspace::resolve_current(current_dir, cli.workspace.as_deref())?
    };
    let target_environment = cli
        .target
        .as_ref()
        .and_then(|target| target.environment.as_ref());
    let backend_override_value = if cli.use_local_backend {
        LOCAL_BACKEND_OVERRIDE_VALUE
    } else {
        "null"
    };
    let logger = Logger::new(cli.verbose);

    if let TargetCommand::Tf(args) = command {
        return run_terraform_target(
            cli,
            args,
            target_environment,
            &cue_bin,
            &workspace,
            backend_override_value,
            &logger,
        );
    }

    let discovered_env;
    let env = if let Some(env) = target_environment {
        env
    } else {
        debug!(logger, "Discovering populated environments");
        discovered_env = environment::discover(&cue_bin, &workspace, backend_override_value)?;
        &discovered_env
    };

    log_target_configuration(&logger, &workspace, env, &cue_bin, &command);

    match command {
        TargetCommand::Cue(command) => run_cue(
            &logger,
            &workspace,
            env,
            &cue_bin,
            backend_override_value,
            command,
        )
        .map(Some),
        TargetCommand::Tf(_) => {
            unreachable!("Terraform commands return before environment selection")
        }
        TargetCommand::Migrate(command) => {
            let tf_bin = if matches!(
                command,
                MigrationCommand::Plan { .. } | MigrationCommand::Apply { .. }
            ) {
                Some(resolve_tool(
                    cli.tf_path
                        .as_deref()
                        .unwrap_or_else(|| std::path::Path::new(DEFAULT_TF_BIN)),
                )?)
            } else {
                None
            };
            MigrationRunner {
                logger: &logger,
                workspace: &workspace,
                env,
                cue_bin: &cue_bin,
                tf_bin: tf_bin.as_deref(),
                tfmigrate_path: cli.tfmigrate_path.as_deref(),
                timeout: cli.timeout,
                backend_override_value,
            }
            .run(command)
        }
    }
}

fn run_terraform_target(
    cli: &Cli,
    args: &[String],
    target_environment: Option<&Env>,
    cue_bin: &std::path::Path,
    workspace: &Workspace,
    backend_override_value: &str,
    logger: &Logger,
) -> Result<Option<ExitStatus>> {
    let tf_bin = resolve_tool(
        cli.tf_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new(DEFAULT_TF_BIN)),
    )?;
    debug!(logger, "Tf Bin: {:?}", tf_bin);
    let desired_environments = environment::populated(cue_bin, workspace, backend_override_value)?;
    let initialized_environments = reconciliation::environment_names(workspace)?;
    let env = if let Some(env) = target_environment {
        env
    } else {
        environment::select(desired_environments.union(&initialized_environments))?
    };
    let reconciliation = reconciliation::inspect(logger, workspace, &tf_bin, env, cli.timeout)?;
    if !desired_environments.contains(env) && reconciliation.is_none() {
        reconciliation::remove_local(workspace, env)?;
        return Err(miette::miette!(
            "Environment '{env}' has no remaining state"
        ));
    }

    log_target_configuration(logger, workspace, env, cue_bin, &TargetCommand::Tf(args));
    let metadata = TerraformMetadata {
        backend_override_value,
        reconciliation: reconciliation.as_ref(),
    };
    let status = run_tf_with_metadata(
        logger,
        workspace,
        env,
        cue_bin,
        &tf_bin,
        &metadata,
        args,
        cli.timeout,
    )?;
    let command = args.iter().find(|argument| !argument.starts_with('-'));
    if status.success()
        && !desired_environments.contains(env)
        && command.is_some_and(|command| matches!(command.as_str(), "apply" | "destroy"))
    {
        reconciliation::remove_if_empty(logger, workspace, &tf_bin, env, cli.timeout)?;
    }
    Ok(Some(status))
}

fn log_target_configuration(
    logger: &Logger,
    workspace: &Workspace,
    env: &Env,
    cue_bin: &std::path::Path,
    command: &TargetCommand<'_>,
) {
    info!(
        logger,
        "Module: {}\nEnvironment: {}",
        workspace.module_name(),
        env
    );
    debug!(
        logger,
        "Configuration:\n    \
            - Cue Bin: {:?}\n    \
            - Root: {:?}\n    \
            - Module Location: {:?}\n    \
            - Module Name: {}\n    \
            - Module Package: {}\n    \
            - Env: {}\n    \
            - Command: {:?}",
        cue_bin,
        workspace.root(),
        workspace.target_dir(),
        workspace.module_name(),
        workspace.module_package(),
        env,
        command
    );
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationMetadata {
    module_history: Vec<String>,
    resource_transitions: Vec<ResourceTransition>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceTransition {
    resource_type: String,
    from: ResourceIdentity,
    to: ResourceIdentity,
}

#[derive(Deserialize, Serialize)]
struct ResourceIdentity {
    module: String,
    env: Env,
    name: String,
}

#[derive(Deserialize)]
struct StateSnapshotMetadata {
    lineage: String,
    serial: u64,
    #[serde(flatten)]
    contents: BTreeMap<String, serde_json::Value>,
}

enum StateSnapshot {
    Missing,
    Present(StateSnapshotMetadata),
}

enum DestinationAction {
    Copy,
    Current,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationEndpoint<'a> {
    module: &'a str,
    environment: &'a str,
    backend: serde_json::Value,
    backend_location_complete: bool,
    lock_file: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleMigrationInspection<'a> {
    kind: &'static str,
    source: MigrationEndpoint<'a>,
    destination: MigrationEndpoint<'a>,
}

#[derive(Serialize)]
struct MigrationDocument<'a> {
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

struct MigrationDirectory(PathBuf);

impl MigrationDirectory {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for MigrationDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct MigrationRunner<'a> {
    logger: &'a Logger,
    workspace: &'a Workspace,
    env: &'a Env,
    cue_bin: &'a std::path::Path,
    tf_bin: Option<&'a std::path::Path>,
    tfmigrate_path: Option<&'a std::path::Path>,
    timeout: Option<Duration>,
    backend_override_value: &'a str,
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

impl MigrationRunner<'_> {
    fn tf_bin(&self) -> Result<&std::path::Path> {
        self.tf_bin
            .ok_or_else(|| miette::miette!("Migration command requires OpenTofu"))
    }

    fn run(&self, command: &MigrationCommand) -> Result<Option<ExitStatus>> {
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
            return self.run_module(command, target);
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
            println!(
                "{}",
                serde_json::to_string_pretty(&metadata).into_diagnostic()?
            );
            return Ok(None);
        }

        let (status, destination_dir) = self.export_destination()?;
        if !status.success() {
            return Ok(Some(status));
        }
        let preparation = self.prepare_resources(&transitions, &destination_dir)?;
        let Preparation::Ready(prepared) = preparation else {
            let Preparation::CommandFailed(status) = preparation else {
                unreachable!()
            };
            return Ok(Some(status));
        };

        let migration = migration_document(&prepared);
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
            self.tf_bin
                .ok_or_else(|| miette::miette!("Migration command requires OpenTofu"))?,
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
                    && transition.to.env == *self.env
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

    fn check_module_layout(&self, source_module: &str, source_env: &Env) -> Result<()> {
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
        source_env: &'m Env,
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

    fn run_module(&self, command: &MigrationCommand, target: &str) -> Result<Option<ExitStatus>> {
        if self.backend_override_value != "null" {
            return Err(miette::miette!(
                "Module migrations cannot be combined with --use-local-backend"
            ));
        }
        let (source_module, source_env) = parse_history_target(target, self.env)?;
        if source_module == self.workspace.module_name() && source_env == *self.env {
            return Err(miette::miette!(
                "The latest module history target is the current target"
            ));
        }
        self.check_module_layout(&source_module, &source_env)?;
        if matches!(command, MigrationCommand::Check) {
            return Ok(None);
        }
        if matches!(command, MigrationCommand::Inspect) {
            let inspection = self.inspect_module(&source_module, &source_env)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&inspection).into_diagnostic()?
            );
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

        let status = self.plan_module(&source_module, &source_env, migration_dir.path())?;
        if !status.success() || matches!(command, MigrationCommand::Plan { .. }) {
            return Ok(Some(status));
        }

        self.apply_module(&source_module, &source_env, migration_dir.path())
            .map(Some)
    }

    fn apply_module(
        &self,
        source_module: &str,
        source_env: &Env,
        migration_dir: &std::path::Path,
    ) -> Result<ExitStatus> {
        let status = self.prepare_module_source(source_module, source_env, migration_dir)?;
        if !status.success() {
            return Ok(status);
        }
        let StateSnapshot::Present(source_metadata) = self.read_state_snapshot(migration_dir)?
        else {
            return Err(miette::miette!(
                "Source state is missing; cannot verify that the migration was previously applied"
            ));
        };

        let destination_dir = MigrationDirectory::new(
            migration_dir.with_file_name(format!("{}.destination", self.env)),
        );
        let status = self.prepare_module_destination(destination_dir.path())?;
        if !status.success() {
            return Ok(status);
        }
        if matches!(
            destination_action(
                &source_metadata,
                &self.read_state_snapshot(destination_dir.path())?
            )?,
            DestinationAction::Current
        ) {
            return run_tf_in(
                self.logger,
                self.tf_bin()?,
                destination_dir.path(),
                &["plan", "-detailed-exitcode", "-lock-timeout=5m"],
                self.timeout,
            );
        }
        let status = export_terraform_to(
            self.logger,
            self.workspace,
            self.env,
            self.cue_bin,
            "null",
            migration_dir,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let status = run_tf_in(
            self.logger,
            self.tf_bin()?,
            migration_dir,
            &[
                "init",
                "-migrate-state",
                "-lockfile=readonly",
                "-lock-timeout=5m",
            ],
            self.timeout,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let destination_state = output_tf_in(
            self.logger,
            self.tf_bin()?,
            migration_dir,
            &["state", "pull"],
            self.timeout,
        )?;
        if !destination_state.status.success() {
            return Ok(destination_state.status);
        }
        let destination_metadata = state_snapshot_metadata(&destination_state.stdout)?;
        ensure_migrated_snapshot(&source_metadata, &destination_metadata)?;
        run_tf_in(
            self.logger,
            self.tf_bin()?,
            migration_dir,
            &["plan", "-detailed-exitcode", "-lock-timeout=5m"],
            self.timeout,
        )
    }

    fn plan_module(
        &self,
        source_module: &str,
        source_env: &Env,
        migration_dir: &std::path::Path,
    ) -> Result<ExitStatus> {
        let status = self.prepare_module_source(source_module, source_env, migration_dir)?;
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
            self.tf_bin()?,
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
            self.tf_bin()?,
            migration_dir,
            &["plan", "-detailed-exitcode", "-lock-timeout=5m"],
            self.timeout,
        )
    }

    fn prepare_module_destination(&self, destination_dir: &std::path::Path) -> Result<ExitStatus> {
        if destination_dir.exists() {
            std::fs::remove_dir_all(destination_dir).into_diagnostic()?;
        }
        let status = export_terraform_to(
            self.logger,
            self.workspace,
            self.env,
            self.cue_bin,
            "null",
            destination_dir,
        )?;
        if !status.success() {
            return Ok(status);
        }
        self.copy_provider_lock(destination_dir)?;
        run_tf_in(
            self.logger,
            self.tf_bin()?,
            destination_dir,
            &["init", "-input=false", "-lockfile=readonly"],
            self.timeout,
        )
    }

    fn read_state_snapshot(&self, directory: &std::path::Path) -> Result<StateSnapshot> {
        let output = capture_tf_in(
            self.logger,
            self.tf_bin()?,
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

    fn prepare_module_source(
        &self,
        source_module: &str,
        source_env: &Env,
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
            self.tf_bin()?,
            migration_dir,
            &["init", "-input=false", "-lockfile=readonly"],
            self.timeout,
        )?;
        if !status.success() {
            return Ok(status);
        }
        let workspaces = output_tf_in(
            self.logger,
            self.tf_bin()?,
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
        let source_workspace = Workspace::resolve(
            self.workspace.root(),
            Some(self.workspace.root()),
            &ModuleTarget::WorkspaceRelative(PathBuf::from(&first.from.module)),
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

fn ensure_default_workspace(output: &[u8]) -> Result<()> {
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

fn ensure_module_migration_files(
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

fn module_lock_path(module: &str, env: &Env) -> String {
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

fn inspected_backend(mut config: serde_json::Value) -> Result<(serde_json::Value, bool)> {
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

fn has_required_providers(config: &serde_json::Value) -> bool {
    config
        .get("terraform")
        .and_then(|terraform| terraform.get("required_providers"))
        .and_then(serde_json::Value::as_object)
        .is_some_and(|providers| !providers.is_empty())
}

fn state_snapshot_missing(stderr: &str) -> bool {
    stderr.contains("No state file was found")
}

fn state_snapshot_metadata(output: &[u8]) -> Result<StateSnapshotMetadata> {
    serde_json::from_slice(output)
        .into_diagnostic()
        .map_err(|error| miette::miette!("Failed to read state snapshot metadata: {error}"))
}

fn ensure_migrated_snapshot(
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

fn destination_action(
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

fn ensure_single_source(transitions: &[&ResourceTransition]) -> Result<()> {
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

fn resource_actions(transitions: &[&ResourceTransition]) -> Vec<String> {
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

fn migration_document(prepared: &PreparedMigration) -> MigrationDocument<'_> {
    MigrationDocument {
        migration: MigrationBody {
            multi_state: MultiStateMigration {
                cuet: CuetMigration {
                    from_dir: &prepared.source_dir,
                    from_skip_plan: prepared.from_skip_plan,
                    to_dir: ".",
                    actions: &prepared.actions,
                },
            },
        },
    }
}

fn parse_history_target(target: &str, default_env: &Env) -> Result<(String, Env)> {
    let (module, env) = target
        .split_once(':')
        .map_or((target, default_env.clone()), |(module, env)| {
            (module, env.to_owned())
        });
    let module = module.strip_prefix('/').unwrap_or(module);
    if module.is_empty() {
        return Err(miette::miette!("History target module cannot be empty"));
    }
    let env = parse_env(&env).map_err(|error| miette::miette!(error))?;
    Ok((module.to_owned(), env))
}

fn run_modules(
    command: &ModulesCommand,
    root: &std::path::Path,
    cue_path: Option<&std::path::Path>,
    use_local_backend: bool,
    verbose: bool,
    output: &mut impl Write,
) -> Result<()> {
    match command {
        ModulesCommand::List => {
            for module in discover_modules(root)? {
                writeln!(output, "{module}").into_diagnostic()?;
            }
            Ok(())
        }
        ModulesCommand::Check => {
            let cue_bin =
                resolve_tool(cue_path.unwrap_or_else(|| std::path::Path::new(DEFAULT_CUE_BIN)))?;
            let backend_override_value = if use_local_backend {
                LOCAL_BACKEND_OVERRIDE_VALUE
            } else {
                "null"
            };
            check_modules(
                &Logger::new(verbose),
                root,
                &cue_bin,
                backend_override_value,
            )
        }
    }
}

fn check_modules(
    logger: &Logger,
    root: &std::path::Path,
    cue_bin: &std::path::Path,
    backend_override_value: &str,
) -> Result<()> {
    let mut failures = Vec::new();
    let workspaces = discover_modules(root)?
        .into_iter()
        .map(|module| {
            let workspace = Workspace::resolve_workspace_relative(
                root,
                Some(root),
                std::path::Path::new(&module),
            )?;
            Ok((module, workspace))
        })
        .collect::<Result<Vec<_>>>()?;
    let workers = thread::available_parallelism().map_or(1, usize::from);
    let mut checks = Vec::new();

    for chunk in workspaces.chunks(workers) {
        let results = thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(_, workspace)| {
                    scope.spawn(|| {
                        environment::populated(cue_bin, workspace, backend_override_value)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| miette::miette!("Environment discovery worker panicked"))
                })
                .collect::<Result<Vec<_>>>()
        })?;

        for ((module, workspace), result) in chunk.iter().zip(results) {
            match result {
                Ok(environments) => {
                    let mut environments: Vec<_> = environments.into_iter().collect();
                    environments.sort();
                    checks.extend(
                        environments
                            .into_iter()
                            .map(|environment| (module.as_str(), workspace, environment)),
                    );
                }
                Err(error) => failures.push(format!("{module}: {error}")),
            }
        }
    }

    for (module, _, environment) in &checks {
        info!(logger, "Checking {module}:{environment}");
    }
    for chunk in checks.chunks(workers) {
        let results = thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(_, workspace, environment)| {
                    scope.spawn(|| {
                        check_cue_export(workspace, environment, cue_bin, backend_override_value)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| miette::miette!("CUE check worker panicked"))
                })
                .collect::<Result<Vec<_>>>()
        })?;

        for ((module, _, environment), result) in chunk.iter().zip(results) {
            match result {
                Ok(output) if output.status.success() => {}
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stderr = stderr.trim();
                    let detail = if stderr.is_empty() {
                        String::new()
                    } else {
                        format!("\n  {}", stderr.replace('\n', "\n  "))
                    };
                    failures.push(format!("{module}:{environment}: {}{detail}", output.status));
                }
                Err(error) => failures.push(format!("{module}:{environment}: {error}")),
            }
        }
    }

    if failures.is_empty() {
        return Ok(());
    }
    failures.sort();

    Err(miette::miette!(
        "Module checks failed:\n- {}",
        failures.join("\n- ")
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        DestinationAction, StateSnapshot, StateSnapshotMetadata, destination_action,
        ensure_default_workspace, ensure_migrated_snapshot, ensure_module_migration_files,
        inspected_backend, parse_history_target, run_from, state_snapshot_missing,
    };
    use crate::cli::{Cli, Commands, MigrationCommand, ModuleTarget, ModulesCommand, Target};
    use crate::test_directory::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};

    fn modules_cli(temp: &TestDirectory) -> Result<Cli> {
        let root = temp.path().join("workspace");
        fs::create_dir(&root).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(root.join("cuet.cue"), "").into_diagnostic()?;
        Ok(Cli {
            verbose: false,
            target: None,
            workspace: Some(root),
            cue_path: Some(temp.path().join("missing-cue")),
            tf_path: Some(temp.path().join("missing-tofu")),
            tfmigrate_path: Some(temp.path().join("missing-tfmigrate")),
            timeout: None,
            use_local_backend: false,
            command: Commands::Modules {
                command: ModulesCommand::List,
            },
        })
    }

    fn module_migration_cli(
        root: &Path,
        cue_bin: &Path,
        tf_bin: &Path,
        temp: &TestDirectory,
        command: MigrationCommand,
    ) -> Cli {
        Cli {
            verbose: false,
            target: Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("new")),
                environment: Some("prod".to_owned()),
            }),
            workspace: Some(root.to_owned()),
            cue_path: Some(cue_bin.to_owned()),
            tf_path: Some(tf_bin.to_owned()),
            tfmigrate_path: Some(temp.path().join("missing-tfmigrate")),
            timeout: None,
            use_local_backend: false,
            command: Commands::Migrate { command },
        }
    }

    fn write_module_migration_tools(
        temp: &TestDirectory,
    ) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
        let cue_bin = temp.path().join("cue");
        let cue_marker = temp.path().join("cue-invocations");
        temp.write_executable(
            &cue_bin,
            &format!(
                r#"#!/usr/bin/env bash
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
printf '%s\n' "$expression" >> '{}'
if [[ $expression == *'#migration'* ]]; then
	printf '{{"moduleHistory":["/old:prod"],"resourceTransitions":[]}}'
	exit 0
fi
if [[ $expression == *'#backends'* ]]; then
	config='{{"terraform":{{"backend":{{"local":{{"path":"historical.tfstate"}}}},"required_version":"0.12.0"}}}}'
elif [[ $expression == *'localBackendOverride: "local.tfstate"'* ]]; then
	config='{{"terraform":{{"backend":{{"local":{{"path":"local.tfstate"}}}},"required_providers":{{"example":{{"source":"example/example","version":"1.0.0"}}}}}},"resource":{{"terraform_data":{{"current":{{}}}}}},"output":{{"current":{{"value":"current"}}}}}}'
else
	config='{{"terraform":{{"backend":{{"local":{{"path":"destination.tfstate"}}}},"required_providers":{{"example":{{"source":"example/example","version":"1.0.0"}}}}}},"resource":{{"terraform_data":{{"current":{{}}}}}},"output":{{"current":{{"value":"current"}}}}}}'
fi
if [[ -n $output ]]; then
	printf '%s' "$config" > "$output"
else
	printf '%s' "$config"
fi
"#,
                cue_marker.display()
            ),
        )?;
        let tf_bin = temp.path().join("tofu");
        let tf_marker = temp.path().join("tofu-invocations");
        temp.write_executable(
            &tf_bin,
            &format!(
                r#"#!/usr/bin/env bash
set -euo pipefail
if [[ $1 == init && ! -f .terraform.lock.hcl ]]; then
	printf 'provider lock missing\n' >&2
	exit 41
fi
printf '%s\n' "$*" >> '{}'
if [[ $* == 'workspace list -no-color' ]]; then
	printf '* default\n'
elif [[ $* == 'state pull' ]]; then
	if [[ $PWD == *.destination ]]; then
		printf 'No state file was found!\n' >&2
		exit 1
	fi
	printf '{{"lineage":"test-lineage","serial":1}}'
fi
"#,
                tf_marker.display()
            ),
        )?;
        Ok((cue_bin, cue_marker, tf_bin, tf_marker))
    }

    #[test]
    fn test_terraform_apply_reconciles_and_removes_historical_environment() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("service");
        let environment_dir = module.join(".cuet/global");
        let unrelated_dir = module.join(".cuet/unrelated");
        fs::create_dir_all(environment_dir.join(".terraform")).into_diagnostic()?;
        fs::create_dir_all(unrelated_dir.join(".terraform")).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        fs::write(environment_dir.join(".terraform/terraform.tfstate"), "").into_diagnostic()?;
        fs::write(unrelated_dir.join(".terraform/terraform.tfstate"), "").into_diagnostic()?;

        let cue_bin = temp.path().join("cue");
        let cue_expression = temp.path().join("cue-expression");
        temp.write_executable(
            &cue_bin,
            &format!(
                r#"#!/usr/bin/env bash
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
if [[ $expression == \[* ]]; then
    printf '[]'
elif [[ -n $output ]]; then
    printf '%s' "$expression" > '{}'
    printf '{{}}' > "$output"
fi
"#,
                cue_expression.display()
            ),
        )?;
        let tf_bin = temp.path().join("tofu");
        let applied = temp.path().join("applied");
        temp.write_executable(
            &tf_bin,
            &format!(
                r#"#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    "state list")
        if [[ $PWD == */unrelated ]]; then exit 99; fi
        if [[ ! -f '{}' ]]; then printf '%s\n' 'neon_project.example'; fi
        ;;
    "output -json") printf '{{}}' ;;
    "apply") touch '{}' ;;
esac
"#,
                applied.display(),
                applied.display()
            ),
        )?;
        let cli = Cli {
            verbose: false,
            target: Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("service")),
                environment: Some("global".to_owned()),
            }),
            workspace: Some(root),
            cue_path: Some(cue_bin),
            tf_path: Some(tf_bin),
            tfmigrate_path: None,
            timeout: None,
            use_local_backend: false,
            command: Commands::Tf {
                args: vec!["apply".to_owned()],
            },
        };

        let status = run_from(&cli, temp.path(), &mut Vec::new())?
            .expect("Terraform command should return a status");

        assert!(status.success());
        assert!(applied.is_file());
        assert!(!environment_dir.exists());
        assert!(unrelated_dir.exists());
        let expression = fs::read_to_string(cue_expression).into_diagnostic()?;
        assert!(
            expression.contains(
                r#"reconciliation: {"environment":"global","requiredProviders":["neon"]}"#
            )
        );
        Ok(())
    }

    #[test]
    fn test_terraform_rejects_selected_historical_environment_without_state() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("service");
        let environment_dir = module.join(".cuet/old");
        fs::create_dir_all(environment_dir.join(".terraform")).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        fs::write(environment_dir.join(".terraform/terraform.tfstate"), "").into_diagnostic()?;
        let cue_bin = temp.path().join("cue");
        temp.write_executable(
            &cue_bin,
            r"#!/usr/bin/env bash
set -euo pipefail
printf '[]'
",
        )?;
        let tf_bin = temp.path().join("tofu");
        let invocations = temp.path().join("tofu-invocations");
        temp.write_executable(
            &tf_bin,
            &format!(
                r#"#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> '{}'
if [[ $* == 'output -json' ]]; then printf '{{}}'; fi
"#,
                invocations.display()
            ),
        )?;
        let cli = Cli {
            verbose: false,
            target: Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("service")),
                environment: Some("old".to_owned()),
            }),
            workspace: Some(root),
            cue_path: Some(cue_bin),
            tf_path: Some(tf_bin),
            tfmigrate_path: None,
            timeout: None,
            use_local_backend: false,
            command: Commands::Tf {
                args: vec!["plan".to_owned()],
            },
        };

        let error = run_from(&cli, temp.path(), &mut Vec::new())
            .expect_err("empty historical environment should be rejected");

        assert!(error.to_string().contains("has no remaining state"));
        assert!(!environment_dir.exists());
        assert_eq!(
            fs::read_to_string(invocations).into_diagnostic()?,
            "state list\noutput -json\n"
        );
        Ok(())
    }

    #[test]
    fn test_modules_requires_no_environment_or_tools() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cli = modules_cli(&temp)?;

        let mut output = Vec::new();
        let status = run_from(&cli, temp.path(), &mut output)?;

        assert!(status.is_none());
        assert_eq!(output, b".\n");
        Ok(())
    }

    #[test]
    fn test_version_requires_no_workspace_or_tools() -> Result<()> {
        let cli = Cli {
            verbose: false,
            target: None,
            workspace: Some(PathBuf::from("/missing-workspace")),
            cue_path: Some(PathBuf::from("/missing-cue")),
            tf_path: Some(PathBuf::from("/missing-tofu")),
            tfmigrate_path: Some(PathBuf::from("/missing-tfmigrate")),
            timeout: None,
            use_local_backend: false,
            command: Commands::Version,
        };

        let mut output = Vec::new();
        let status = run_from(&cli, Path::new("/missing-directory"), &mut output)?;

        assert!(status.is_none());
        assert_eq!(
            output,
            format!("cuet {}\n", env!("CARGO_PKG_VERSION")).as_bytes()
        );
        Ok(())
    }

    #[test]
    fn test_history_target_defaults_environment_and_accepts_workspace_prefix() -> Result<()> {
        assert_eq!(
            parse_history_target("/infra/old", &"prod".to_owned())?,
            ("infra/old".to_owned(), "prod".to_owned())
        );
        assert_eq!(
            parse_history_target("infra/old:dev", &"prod".to_owned())?,
            ("infra/old".to_owned(), "dev".to_owned())
        );
        Ok(())
    }

    #[test]
    fn test_module_migration_requires_only_default_workspace() {
        ensure_default_workspace(b"* default\n").unwrap();

        let error = ensure_default_workspace(b"* default\n  development\n")
            .expect_err("additional workspaces should be rejected");

        assert!(error.to_string().contains("default, development"));
    }

    #[test]
    fn test_module_migration_requires_destination_snapshot() {
        let source = StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 4,
            contents: BTreeMap::new(),
        };
        let current_destination = StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 5,
            contents: BTreeMap::new(),
        };
        let stale_destination = StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 3,
            contents: BTreeMap::new(),
        };

        ensure_migrated_snapshot(&source, &current_destination).unwrap();
        let error = ensure_migrated_snapshot(&source, &stale_destination)
            .expect_err("stale destination should be rejected");

        assert!(error.to_string().contains("migrated source snapshot"));
    }

    #[test]
    fn test_module_migration_guards_destination_state() {
        let source = StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 4,
            contents: BTreeMap::new(),
        };
        let current = StateSnapshot::Present(StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 4,
            contents: BTreeMap::new(),
        });
        let stale = StateSnapshot::Present(StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 3,
            contents: BTreeMap::new(),
        });
        let unrelated = StateSnapshot::Present(StateSnapshotMetadata {
            lineage: "other".to_owned(),
            serial: 5,
            contents: BTreeMap::new(),
        });
        let divergent = StateSnapshot::Present(StateSnapshotMetadata {
            lineage: "source".to_owned(),
            serial: 4,
            contents: BTreeMap::from([("outputs".to_owned(), serde_json::json!({"x": 1}))]),
        });

        assert!(matches!(
            destination_action(&source, &StateSnapshot::Missing).unwrap(),
            DestinationAction::Copy
        ));
        assert!(destination_action(&source, &stale).is_err());
        assert!(matches!(
            destination_action(&source, &current).unwrap(),
            DestinationAction::Current
        ));
        assert!(destination_action(&source, &unrelated).is_err());
        assert!(destination_action(&source, &divergent).is_err());
        assert!(state_snapshot_missing("Error: No state file was found!"));
        assert!(!state_snapshot_missing("permission denied"));
    }

    #[test]
    fn test_module_migration_checks_repository_files() -> Result<()> {
        let temp = TestDirectory::new()?;
        let source_lock = temp.path().join("old/.cuet/prod/.terraform.lock.hcl");
        let destination_lock = temp.path().join("new/.cuet/prod/.terraform.lock.hcl");
        let output_dir = temp.path().join("new/.cuet/prod");
        let scratch_dir = temp.path().join("new/.cuet/prod.migrate");
        fs::create_dir_all(
            source_lock
                .parent()
                .expect("source lock should have parent"),
        )
        .into_diagnostic()?;
        fs::write(&source_lock, "").into_diagnostic()?;

        let lock_error = ensure_module_migration_files(
            &source_lock,
            &destination_lock,
            &output_dir,
            &scratch_dir,
        )
        .expect_err("historical lock should be rejected");
        fs::remove_file(&source_lock).into_diagnostic()?;
        fs::create_dir_all(&output_dir).into_diagnostic()?;
        fs::write(output_dir.join("tfmigrate.json"), "").into_diagnostic()?;
        let artifact_error = ensure_module_migration_files(
            &source_lock,
            &destination_lock,
            &output_dir,
            &scratch_dir,
        )
        .expect_err("tfmigrate artifact should be rejected");

        assert!(lock_error.to_string().contains("move it"));
        assert!(
            artifact_error
                .to_string()
                .contains("Stale tfmigrate artifact")
        );
        Ok(())
    }

    #[test]
    fn test_migration_inspection_redacts_backend_secrets() {
        let config = serde_json::json!({
            "terraform": {"backend": {"gcs": {
                "bucket": "state",
                "credentials": "secret-json",
                "conn_str": "postgres://secret"
            }}},
        });

        let (backend, complete) = inspected_backend(config).unwrap();

        assert!(complete);
        assert_eq!(backend["gcs"]["bucket"], "state");
        assert!(backend["gcs"].get("credentials").is_none());
        assert!(backend["gcs"].get("conn_str").is_none());
    }

    #[test]
    fn test_modules_reports_output_failure() -> Result<()> {
        struct BrokenPipe;

        impl Write for BrokenPipe {
            fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let temp = TestDirectory::new()?;
        let cli = modules_cli(&temp)?;

        let error = run_from(&cli, temp.path(), &mut BrokenPipe)
            .expect_err("output failure should be reported");

        assert!(error.to_string().to_lowercase().contains("broken pipe"));
        Ok(())
    }

    #[test]
    fn test_modules_check_aggregates_failures_and_continues() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        for module in ["alpha", "beta", "gamma"] {
            let directory = root.join(module);
            fs::create_dir_all(&directory).into_diagnostic()?;
            fs::write(directory.join("cuet.cue"), "").into_diagnostic()?;
        }
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        let cue_bin = temp.path().join("cue");
        let marker = temp.path().join("cue-ran");
        temp.write_executable(
            &cue_bin,
            &format!(
                r#"#!/usr/bin/env bash
# Fake CUE records every query and returns module-specific results.
set -euo pipefail
expression=""
while [[ $# -gt 0 ]]; do
	case $1 in
	-e)
		expression=$2
		shift 2
		;;
	*) shift ;;
	esac
done
module=${{PWD##*/}}
printf '%s:%s\n' "$module" "$expression" >> '{}'
if [[ $expression == *'["in"]'* ]]; then
	case $module in
	alpha) printf '["prod","dev"]' ;;
	beta)
		printf 'invalid environments' >&2
		exit 11
		;;
	gamma) printf '["stage"]' ;;
	esac
	exit 0
fi
if [[ $module == alpha && $expression == *'["prod"]'* ]]; then
	printf 'first export error\nsecond export error\n' >&2
	exit 7
fi
"#,
                marker.display()
            ),
        )?;
        let cli = Cli {
            verbose: false,
            target: None,
            workspace: Some(root),
            cue_path: Some(cue_bin),
            tf_path: Some(temp.path().join("missing-tofu")),
            tfmigrate_path: Some(temp.path().join("missing-tfmigrate")),
            timeout: None,
            use_local_backend: false,
            command: Commands::Modules {
                command: ModulesCommand::Check,
            },
        };

        let error = run_from(&cli, temp.path(), &mut Vec::new())
            .expect_err("invalid modules should be reported");

        let message = error.to_string();
        assert!(message.contains("alpha:prod: exit status: 7"));
        assert!(message.contains("first export error"));
        assert!(message.contains("second export error"));
        assert!(message.contains("beta: Failed to discover populated environments"));
        assert!(message.contains("invalid environments"));
        let invocations = fs::read_to_string(marker).into_diagnostic()?;
        assert!(invocations.contains(r#"["dev"]"#));
        assert!(invocations.contains(r#"["prod"]"#));
        assert!(invocations.matches("gamma:").count() >= 2);
        Ok(())
    }

    #[test]
    fn test_modules_check_succeeds_without_terraform() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cue_bin = temp.path().join("cue");
        temp.write_executable(
            &cue_bin,
            r#"#!/usr/bin/env bash
# Fake CUE returns one environment and accepts its export.
set -euo pipefail
while [[ $# -gt 0 ]]; do
	if [[ $1 == -e ]]; then
		expression=$2
		break
	fi
	shift
done
if [[ $expression == *'["in"]'* ]]; then
	printf '["dev"]'
fi
"#,
        )?;
        let mut cli = modules_cli(&temp)?;
        cli.cue_path = Some(cue_bin);
        cli.command = Commands::Modules {
            command: ModulesCommand::Check,
        };

        let status = run_from(&cli, temp.path(), &mut Vec::new())?;

        assert!(status.is_none());
        Ok(())
    }

    #[test]
    fn test_tfmigrate_derives_move_from_history() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let from_module = root.join("from");
        let to_module = root.join("to");
        fs::create_dir_all(&from_module).into_diagnostic()?;
        fs::create_dir_all(&to_module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(from_module.join("cuet.cue"), "").into_diagnostic()?;
        fs::write(to_module.join("cuet.cue"), "").into_diagnostic()?;
        let cue_bin = temp.path().join("cue");
        temp.write_executable(
            &cue_bin,
            r#"#!/usr/bin/env bash
# Fake CUE returns migration history or writes a requested export file.
set -euo pipefail
expression=""
output=""
while [[ $# -gt 0 ]]; do
	case $1 in
	-e)
		expression=$2
		shift 2
		;;
	-o)
		output=$2
		shift 2
		;;
	*) shift ;;
	esac
done
if [[ $expression == *'["in"]'* ]]; then
	printf '["prod"]'
	exit 0
fi
if [[ $expression == *'#migration'* ]]; then
	printf '{"moduleHistory":[],"resourceTransitions":[{"resourceType":"terraform_data","from":{"module":"from","env":"prod","name":"old"},"to":{"module":"to","env":"prod","name":"new"}}]}'
	exit 0
fi
printf '{}' > "$output"
"#,
        )?;
        let tf_bin = temp.path().join("tofu");
        temp.write_executable(
            &tf_bin,
            "#!/usr/bin/env bash\n# Fake OpenTofu is resolved but run by tfmigrate.\nset -euo pipefail\n",
        )?;
        let tfmigrate_bin = temp.path().join("tfmigrate");
        let migration_marker = temp.path().join("migration.json");
        temp.write_executable(
            &tfmigrate_bin,
            &format!(
                r#"#!/usr/bin/env bash
# Fake tfmigrate preserves the generated migration.
set -euo pipefail
cp "$2" '{}'
"#,
                migration_marker.display()
            ),
        )?;
        let cli = Cli {
            verbose: false,
            target: Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("to")),
                environment: Some("prod".to_owned()),
            }),
            workspace: Some(root),
            cue_path: Some(cue_bin),
            tf_path: Some(tf_bin),
            tfmigrate_path: Some(tfmigrate_bin),
            timeout: None,
            use_local_backend: false,
            command: Commands::Migrate {
                command: MigrationCommand::Plan { args: Vec::new() },
            },
        };

        let status = run_from(&cli, temp.path(), &mut Vec::new())?
            .expect("migration should return its process status");

        assert!(status.success());
        assert!(from_module.join(".cuet/prod/main.tf.json").is_file());
        assert!(to_module.join(".cuet/prod/main.tf.json").is_file());
        assert!(to_module.join(".cuet/prod/tfmigrate.json").is_file());
        let migration: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(migration_marker).into_diagnostic()?)
                .into_diagnostic()?;
        assert_eq!(
            migration["migration"]["multi_state"]["cuet"]["actions"][0],
            "mv terraform_data.old terraform_data.new"
        );
        Ok(())
    }

    #[test]
    fn test_module_migration_uses_native_backend_migration() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("new");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        fs::create_dir_all(module.join(".cuet/prod")).into_diagnostic()?;
        fs::write(
            module.join(".cuet/prod/.terraform.lock.hcl"),
            "# provider lock\n",
        )
        .into_diagnostic()?;
        let (cue_bin, cue_marker, tf_bin, tf_marker) = write_module_migration_tools(&temp)?;
        let cli = module_migration_cli(&root, &cue_bin, &tf_bin, &temp, MigrationCommand::Check);

        let status = run_from(&cli, temp.path(), &mut Vec::new())?;

        assert!(status.is_none());
        assert!(!tf_marker.exists());
        let cli = module_migration_cli(
            &root,
            &cue_bin,
            &tf_bin,
            &temp,
            MigrationCommand::Plan { args: Vec::new() },
        );

        let status = run_from(&cli, temp.path(), &mut Vec::new())?
            .expect("migration should return its process status");

        assert!(status.success());
        assert_eq!(
            fs::read_to_string(&tf_marker).into_diagnostic()?,
            "init -input=false -lockfile=readonly\nworkspace list -no-color\nstate pull\ninit -migrate-state -force-copy -input=false -lockfile=readonly -lock-timeout=5m\nplan -detailed-exitcode -lock-timeout=5m\n"
        );
        let cue_invocations = fs::read_to_string(cue_marker).into_diagnostic()?;
        assert!(cue_invocations.contains(r#"#backends)["prod"]"#));
        assert!(cue_invocations.contains(r#"module: "old""#));
        assert!(!cue_invocations.contains("backendModule"));
        assert!(!cue_invocations.contains("backendEnvironment"));
        assert!(cue_invocations.contains(r#"localBackendOverride: "local.tfstate""#));
        assert!(!module.join(".cuet/prod.migrate").exists());

        fs::write(&tf_marker, "").into_diagnostic()?;
        let cli = module_migration_cli(
            &root,
            &cue_bin,
            &tf_bin,
            &temp,
            MigrationCommand::Apply { args: Vec::new() },
        );

        let status = run_from(&cli, temp.path(), &mut Vec::new())?
            .expect("migration should return its process status");

        assert!(status.success());
        assert_eq!(
            fs::read_to_string(&tf_marker).into_diagnostic()?,
            "init -input=false -lockfile=readonly\nworkspace list -no-color\nstate pull\ninit -migrate-state -force-copy -input=false -lockfile=readonly -lock-timeout=5m\nplan -detailed-exitcode -lock-timeout=5m\ninit -input=false -lockfile=readonly\nworkspace list -no-color\nstate pull\ninit -input=false -lockfile=readonly\nstate pull\ninit -migrate-state -lockfile=readonly -lock-timeout=5m\nstate pull\nplan -detailed-exitcode -lock-timeout=5m\n"
        );
        Ok(())
    }
}
