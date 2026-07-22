use crate::cli::{Cli, Commands, Env, ModuleTarget, ModulesCommand, TfmigrateCommand, parse_env};
use crate::environment;
use crate::execution::{
    check_cue_export, export_historical_backend, export_terraform, read_tfmigrate_metadata,
    resolve_tool, run_cue, run_tf, run_tfmigrate,
};
use crate::logger::Logger;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use miette::{IntoDiagnostic, Result};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::thread;

const DEFAULT_CUE_BIN: &str = "cue";
const DEFAULT_TF_BIN: &str = "tofu";
const DEFAULT_TFMIGRATE_BIN: &str = "tfmigrate";
const LOCAL_STATE_FILE_NAME: &str = "local.tfstate";

struct TargetInvocation {
    verbose: bool,
    target: Option<crate::cli::Target>,
    workspace_root: Option<PathBuf>,
    cue_path: Option<PathBuf>,
    tf_path: Option<PathBuf>,
    tfmigrate_path: Option<PathBuf>,
    use_local_backend: bool,
    command: Commands,
}

pub fn run(cli: Cli) -> Result<Option<ExitStatus>> {
    let current_dir = std::env::current_dir().into_diagnostic()?;
    run_from(cli, &current_dir, &mut io::stdout().lock())
}

fn run_from(
    cli: Cli,
    current_dir: &std::path::Path,
    output: &mut impl Write,
) -> Result<Option<ExitStatus>> {
    let Cli {
        verbose,
        target,
        workspace: workspace_root,
        cue_path,
        tf_path,
        tfmigrate_path,
        use_local_backend,
        command,
    } = cli;

    if let Commands::Modules { command } = command {
        let root = resolve_root(current_dir, workspace_root.as_deref())?;
        run_modules(
            &command,
            &root,
            cue_path,
            use_local_backend,
            verbose,
            output,
        )?;
        return Ok(None);
    }

    run_target_command(
        TargetInvocation {
            verbose,
            target,
            workspace_root,
            cue_path,
            tf_path,
            tfmigrate_path,
            use_local_backend,
            command,
        },
        current_dir,
    )
}

fn run_target_command(
    invocation: TargetInvocation,
    current_dir: &std::path::Path,
) -> Result<Option<ExitStatus>> {
    let target = invocation.target.unwrap_or_default();
    let cue_bin = resolve_tool(
        &invocation
            .cue_path
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CUE_BIN)),
    )?;
    let workspace = Workspace::resolve(
        current_dir,
        invocation.workspace_root.as_deref(),
        &target.module,
    )?;
    let backend_override_value = if invocation.use_local_backend {
        Cow::Owned(format!("\"{LOCAL_STATE_FILE_NAME}\""))
    } else {
        Cow::Borrowed("null")
    };
    let logger = Logger::new(invocation.verbose);
    let env = target.environment.map_or_else(
        || {
            debug!(logger, "Discovering populated environments");
            environment::discover(&cue_bin, &workspace, &backend_override_value)
        },
        Ok,
    )?;

    log_target_configuration(&logger, &workspace, &env, &cue_bin, &invocation.command);

    match invocation.command {
        Commands::Cue { command } => run_cue(
            &logger,
            &workspace,
            &env,
            &cue_bin,
            &backend_override_value,
            &command,
        )
        .map(Some),
        Commands::Tf { args } => {
            let tf_bin = resolve_tool(
                &invocation
                    .tf_path
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_TF_BIN)),
            )?;
            debug!(logger, "Tf Bin: {:?}", tf_bin);
            run_tf(
                &logger,
                &workspace,
                &env,
                &cue_bin,
                &tf_bin,
                &backend_override_value,
                &args,
            )
            .map(Some)
        }
        Commands::Tfmigrate { command } => {
            let tf_bin = resolve_tool(
                &invocation
                    .tf_path
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_TF_BIN)),
            )?;
            let tfmigrate_bin = resolve_tool(
                &invocation
                    .tfmigrate_path
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_TFMIGRATE_BIN)),
            )?;
            TfmigrateRunner {
                logger: &logger,
                workspace: &workspace,
                env: &env,
                cue_bin: &cue_bin,
                tf_bin: &tf_bin,
                tfmigrate_bin: &tfmigrate_bin,
                backend_override_value: &backend_override_value,
            }
            .run(&command)
            .map(Some)
        }
        Commands::Modules { .. } => {
            unreachable!("modules command handled before tool resolution")
        }
    }
}

fn log_target_configuration(
    logger: &Logger,
    workspace: &Workspace,
    env: &Env,
    cue_bin: &std::path::Path,
    command: &Commands,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationMetadata {
    module_history: Vec<String>,
    resource_transitions: Vec<ResourceTransition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceTransition {
    resource_type: String,
    from: ResourceIdentity,
    to: ResourceIdentity,
}

#[derive(Deserialize)]
struct ResourceIdentity {
    module: String,
    env: Env,
    name: String,
}

struct TfmigrateRunner<'a> {
    logger: &'a Logger,
    workspace: &'a Workspace,
    env: &'a Env,
    cue_bin: &'a std::path::Path,
    tf_bin: &'a std::path::Path,
    tfmigrate_bin: &'a std::path::Path,
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

impl TfmigrateRunner<'_> {
    fn run(&self, command: &TfmigrateCommand) -> Result<ExitStatus> {
        let metadata: MigrationMetadata = serde_json::from_value(read_tfmigrate_metadata(
            self.workspace,
            self.env,
            self.cue_bin,
            self.backend_override_value,
        )?)
        .into_diagnostic()?;
        let transitions = self.current_transitions(&metadata);
        if !metadata.module_history.is_empty() && !transitions.is_empty() {
            return Err(miette::miette!(
                "Module history and cross-state resource history cannot be migrated together"
            ));
        }

        let (status, destination_dir) = self.export_destination()?;
        if !status.success() {
            return Ok(status);
        }
        let preparation = if let Some(target) = metadata.module_history.last() {
            self.prepare_module(target, &destination_dir)?
        } else {
            self.prepare_resources(&transitions, &destination_dir)?
        };
        let Preparation::Ready(prepared) = preparation else {
            let Preparation::CommandFailed(status) = preparation else {
                unreachable!()
            };
            return Ok(status);
        };

        let migration = migration_json(&prepared);
        let (operation, args) = command.command_and_args();
        run_tfmigrate(
            self.logger,
            self.tfmigrate_bin,
            self.tf_bin,
            operation,
            args,
            &destination_dir,
            &migration,
        )
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

    fn prepare_module(
        &self,
        target: &str,
        destination_dir: &std::path::Path,
    ) -> Result<Preparation> {
        let (source_module, source_env) = parse_history_target(target, self.env)?;
        if source_module == self.workspace.module_name() && source_env == *self.env {
            return Err(miette::miette!(
                "The latest module history target is the current target"
            ));
        }
        let source_dir = destination_dir.join(".tfmigrate/from");
        let status = export_historical_backend(
            self.logger,
            self.workspace,
            &source_module,
            &source_env,
            self.cue_bin,
            self.backend_override_value,
            &source_dir,
        )?;
        if !status.success() {
            return Ok(Preparation::CommandFailed(status));
        }
        Ok(Preparation::Ready(PreparedMigration {
            source_dir,
            actions: vec!["xmv * $1".to_owned()],
            from_skip_plan: true,
        }))
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

fn migration_json(prepared: &PreparedMigration) -> serde_json::Value {
    serde_json::json!({
        "migration": {
            "multi_state": {
                "cuet": {
                    "from_dir": prepared.source_dir,
                    "from_skip_plan": prepared.from_skip_plan,
                    "to_dir": ".",
                    "actions": prepared.actions,
                }
            }
        }
    })
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
    cue_path: Option<PathBuf>,
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
                resolve_tool(&cue_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CUE_BIN)))?;
            let backend_override_value = if use_local_backend {
                Cow::Owned(format!("\"{LOCAL_STATE_FILE_NAME}\""))
            } else {
                Cow::Borrowed("null")
            };
            check_modules(
                &Logger::new(verbose),
                root,
                &cue_bin,
                &backend_override_value,
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
            let workspace = Workspace::resolve(
                root,
                Some(root),
                &ModuleTarget::WorkspaceRelative(PathBuf::from(&module)),
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
    use super::{parse_history_target, run_from};
    use crate::cli::{Cli, Commands, ModuleTarget, ModulesCommand, Target, TfmigrateCommand};
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::fs::{self, File};
    use std::io::{self, Write};
    use std::os::unix::fs::PermissionsExt;
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
            use_local_backend: false,
            command: Commands::Modules {
                command: ModulesCommand::List,
            },
        })
    }

    fn write_executable(path: &Path, body: &str) -> Result<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(body.as_bytes()).into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        drop(file);
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).into_diagnostic()
    }

    #[test]
    fn test_modules_requires_no_environment_or_tools() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cli = modules_cli(&temp)?;

        let mut output = Vec::new();
        let status = run_from(cli, temp.path(), &mut output)?;

        assert!(status.is_none());
        assert_eq!(output, b".\n");
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

        let error = run_from(cli, temp.path(), &mut BrokenPipe)
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
        write_executable(
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
if [[ $module == alpha && $expression == *'["prod"]' ]]; then
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
            use_local_backend: false,
            command: Commands::Modules {
                command: ModulesCommand::Check,
            },
        };

        let error = run_from(cli, temp.path(), &mut Vec::new())
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
        write_executable(
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

        let status = run_from(cli, temp.path(), &mut Vec::new())?;

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
        write_executable(
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
        write_executable(
            &tf_bin,
            "#!/usr/bin/env bash\n# Fake OpenTofu is resolved but run by tfmigrate.\nset -euo pipefail\n",
        )?;
        let tfmigrate_bin = temp.path().join("tfmigrate");
        let migration_marker = temp.path().join("migration.json");
        write_executable(
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
            use_local_backend: false,
            command: Commands::Tfmigrate {
                command: TfmigrateCommand::Plan { args: Vec::new() },
            },
        };

        let status = run_from(cli, temp.path(), &mut Vec::new())?
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
    fn test_tfmigrate_generates_backend_only_source_for_module_history() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("new");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        let cue_bin = temp.path().join("cue");
        write_executable(
            &cue_bin,
            r#"#!/usr/bin/env bash
# Fake CUE returns module history and writes generated configurations.
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
if [[ $expression == *'#migration'* ]]; then
	printf '{"moduleHistory":["/old:prod"],"resourceTransitions":[]}'
	exit 0
fi
printf '{}' > "$output"
"#,
        )?;
        let tf_bin = temp.path().join("tofu");
        write_executable(
            &tf_bin,
            "#!/usr/bin/env bash\n# Fake OpenTofu is resolved but run by tfmigrate.\nset -euo pipefail\n",
        )?;
        let tfmigrate_bin = temp.path().join("tfmigrate");
        let migration_marker = temp.path().join("module-migration.json");
        write_executable(
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
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("new")),
                environment: Some("prod".to_owned()),
            }),
            workspace: Some(root),
            cue_path: Some(cue_bin),
            tf_path: Some(tf_bin),
            tfmigrate_path: Some(tfmigrate_bin),
            use_local_backend: false,
            command: Commands::Tfmigrate {
                command: TfmigrateCommand::Plan { args: Vec::new() },
            },
        };

        let status = run_from(cli, temp.path(), &mut Vec::new())?
            .expect("migration should return its process status");

        assert!(status.success());
        assert!(
            module
                .join(".cuet/prod/.tfmigrate/from/main.tf.json")
                .is_file()
        );
        let migration: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(migration_marker).into_diagnostic()?)
                .into_diagnostic()?;
        assert_eq!(
            migration["migration"]["multi_state"]["cuet"]["actions"][0],
            "xmv * $1"
        );
        assert_eq!(
            migration["migration"]["multi_state"]["cuet"]["from_skip_plan"],
            true
        );
        Ok(())
    }
}
