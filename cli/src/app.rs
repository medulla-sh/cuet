use crate::cli::{Cli, Commands, ModulesCommand};
use crate::environment;
use crate::execution::{resolve_tool, run_cue, run_tf};
use crate::logger::Logger;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitStatus;

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

    if matches!(
        &command,
        Commands::Modules {
            command: ModulesCommand::List
        }
    ) {
        let root = resolve_root(current_dir, workspace_root.as_deref())?;
        for module in discover_modules(&root)? {
            writeln!(output, "{module}").into_diagnostic()?;
        }
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

#[cfg(test)]
mod tests {
    use super::run_from;
    use crate::cli::{Cli, Commands, ModulesCommand};
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::fs;
    use std::io::{self, Write};

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
}
