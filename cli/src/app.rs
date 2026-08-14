use crate::cli::{Cli, Commands, CueCommand, MigrationCommand, ModulesCommand};
use crate::completions;
use crate::environment;
use crate::execution::{
    TerraformMetadata, check_cue_export, resolve_tool, run_cue, run_tf_with_metadata,
};
use crate::logger::Logger;
use crate::migration::MigrationRunner;
use crate::reconciliation;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use clap::CommandFactory;
use miette::{IntoDiagnostic, Result};
use std::io::{self, Write};
use std::process::ExitStatus;
use std::thread;

const DEFAULT_CUE_BIN: &str = "cue";
const DEFAULT_TF_BIN: &str = "tofu";
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
            run_target_command(cli, TargetCommand::Cue(command), current_dir, output)
        }
        Commands::Tf { args } => {
            run_target_command(cli, TargetCommand::Tf(args), current_dir, output)
        }
        Commands::Migrate { command } => {
            run_target_command(cli, TargetCommand::Migrate(command), current_dir, output)
        }
    }
}

fn run_target_command(
    cli: &Cli,
    command: TargetCommand<'_>,
    current_dir: &std::path::Path,
    output: &mut impl Write,
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
        .and_then(|target| target.environment.as_deref());
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
        discovered_env.as_str()
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
            MigrationRunner::new(
                &logger,
                &workspace,
                env,
                &cue_bin,
                tf_bin.as_deref(),
                cli.tfmigrate_path.as_deref(),
                cli.timeout,
                backend_override_value,
            )
            .run(command, output)
        }
    }
}

fn run_terraform_target(
    cli: &Cli,
    args: &[String],
    target_environment: Option<&str>,
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
        environment::select(
            desired_environments
                .union(&initialized_environments)
                .map(String::as_str),
        )?
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
    env: &str,
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
    use super::run_from;
    use crate::cli::{Cli, Commands, MigrationCommand, ModuleTarget, ModulesCommand, Target};
    use crate::migration::{
        DestinationAction, StateSnapshot, StateSnapshotMetadata, destination_action,
        ensure_default_workspace, ensure_migrated_snapshot, ensure_module_migration_files,
        inspected_backend, parse_history_target, state_snapshot_missing,
    };
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
	printf '{{"version":4,"lineage":"test-lineage","serial":1}}'
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
    "state pull")
        if [[ $PWD == */unrelated ]]; then exit 99; fi
        if [[ ! -f '{}' ]]; then
            printf '%s' '{{"version":4,"resources":[{{"type":"neon_project","provider":"provider[\"registry.opentofu.org/kislerdm/neon\"].readonly"}}]}}'
        else
            printf '{{"version":4}}'
        fi
        ;;
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
                r#"reconciliation: {"environment":"global","requiredProviders":[{"source":"kislerdm/neon","alias":"readonly"}]}"#
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
if [[ $* == 'state pull' ]]; then printf '{{"version":4}}'; fi
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
            "state pull\n"
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
    fn test_migration_inspect_writes_to_output_sink() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("service");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        let cue_bin = temp.path().join("cue");
        temp.write_executable(
            &cue_bin,
            r#"#!/usr/bin/env bash
set -euo pipefail
printf '%s' '{"moduleHistory":[],"resourceTransitions":[{"resourceType":"neon_project","from":{"module":"old","env":"dev","name":"old"},"to":{"module":"service","env":"dev","name":"new"}}]}'
"#,
        )?;
        let cli = Cli {
            verbose: false,
            target: Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("service")),
                environment: Some("dev".to_owned()),
            }),
            workspace: Some(root),
            cue_path: Some(cue_bin),
            tf_path: None,
            tfmigrate_path: None,
            timeout: None,
            use_local_backend: false,
            command: Commands::Migrate {
                command: MigrationCommand::Inspect,
            },
        };
        let mut output = Vec::new();

        let status = run_from(&cli, temp.path(), &mut output)?;

        assert!(status.is_none());
        assert!(output.ends_with(b"\n"));
        let inspection: serde_json::Value = serde_json::from_slice(&output).into_diagnostic()?;
        assert_eq!(
            inspection["resourceTransitions"][0]["to"]["module"],
            "service"
        );
        Ok(())
    }

    #[test]
    fn test_history_target_defaults_environment_and_accepts_workspace_prefix() -> Result<()> {
        assert_eq!(
            parse_history_target("/infra/old", "prod")?,
            ("infra/old", "prod")
        );
        assert_eq!(
            parse_history_target("infra/old:dev", "prod")?,
            ("infra/old", "dev")
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
