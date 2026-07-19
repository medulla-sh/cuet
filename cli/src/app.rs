use crate::cli::{Cli, Commands};
use crate::environment;
use crate::execution::{resolve_tool, run_cue, run_tf};
use crate::logger::Logger;
use crate::workspace::Workspace;
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;
use std::path::PathBuf;
use std::process::ExitStatus;

const DEFAULT_CUE_BIN: &str = "cue";
const DEFAULT_TF_BIN: &str = "tofu";
const LOCAL_STATE_FILE_NAME: &str = "local.tfstate";

pub fn run(cli: Cli) -> Result<ExitStatus> {
    let Cli {
        verbose,
        target,
        workspace: workspace_root,
        cue_path,
        tf_path,
        use_local_backend,
        command,
    } = cli;

    let target = target.unwrap_or_default();
    let cue_bin = resolve_tool(&cue_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CUE_BIN)))?;
    let tf_bin = resolve_tool(&tf_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TF_BIN)))?;
    let current_dir = std::env::current_dir().into_diagnostic()?;
    let workspace = Workspace::resolve(&current_dir, workspace_root.as_deref(), &target.module)?;
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
            - Tf Bin: {:?}\n    \
            - Root: {:?}\n    \
            - Module Location: {:?}\n    \
            - Module Name: {}\n    \
            - Module Package: {}\n    \
            - Env: {}\n    \
            - Command: {:?}",
        cue_bin,
        tf_bin,
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
        ),
        Commands::Tf { args } => run_tf(
            &logger,
            &workspace,
            &env,
            &cue_bin,
            &tf_bin,
            &backend_override_value,
            &args,
        ),
    }
}
