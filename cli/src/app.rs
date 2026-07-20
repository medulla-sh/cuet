use crate::cli::{Cli, Commands, ModuleTarget, ModulesCommand};
use crate::environment;
use crate::execution::{check_cue_export, resolve_tool, run_cue, run_tf};
use crate::logger::Logger;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::thread;

const DEFAULT_CUE_BIN: &str = "cue";
const DEFAULT_TF_BIN: &str = "tofu";
const LOCAL_STATE_FILE_NAME: &str = "local.tfstate";

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

    let target = target.unwrap_or_default();
    let cue_bin = resolve_tool(&cue_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CUE_BIN)))?;
    let workspace = Workspace::resolve(current_dir, workspace_root.as_deref(), &target.module)?;
    let backend_override_value = if use_local_backend {
        Cow::Owned(format!("\"{LOCAL_STATE_FILE_NAME}\""))
    } else {
        Cow::Borrowed("null")
    };
    let logger = Logger::new(verbose);
    let env = target.environment.map_or_else(
        || {
            debug!(logger, "Discovering populated environments");
            environment::discover(&cue_bin, &workspace, &backend_override_value)
        },
        Ok,
    )?;

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

    match command {
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
            let tf_bin = resolve_tool(&tf_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TF_BIN)))?;
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
        Commands::Modules { .. } => {
            unreachable!("modules command handled before tool resolution")
        }
    }
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
    use super::run_from;
    use crate::cli::{Cli, Commands, ModulesCommand};
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::fs::{self, File};
    use std::io::{self, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

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
}
