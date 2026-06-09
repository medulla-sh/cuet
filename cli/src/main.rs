use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

mod logger;

const DEFAULT_CUE_BIN: &str = "cue";
const DEFAULT_TF_BIN: &str = "tofu";
const OUTPUT_FOLDER_NAME: &str = ".cuet";
const LOCAL_STATE_FILE_NAME: &str = "local.tfstate";
const OUTPUT_FILE_NAME: &str = "main.tf.json";

macro_rules! EXPORT_EXPRESSION {
    () => {
        r#"((infra & {{ #metadata: {{ module: "{}", localBackendOverride: {} }} }}).out)["{}"]"#
    };
}

#[derive(Parser, Debug)]
#[command(name = "cuet")]
#[command(about = "A CLI for interacting with CUE-based Terraform setups")]
struct Cli {
    /// Verbosity
    #[arg(short = 'v', long, global = true)]
    verbose: bool,

    /// Path to the infra module to evaluate. Defaults to current directory.
    #[arg(short = 'p', long)]
    path: Option<PathBuf>,

    /// Path to the 'cue' binary
    #[arg(long)]
    cue_path: Option<PathBuf>,

    /// Path to the 'tofu' (or terraform) binary
    #[arg(long)]
    tf_path: Option<PathBuf>,

    /// Environment name
    #[arg(required = true)]
    env: String,

    /// If set, will override to use a local backend instead of the framework configured backend.
    /// This is useful when creating a backend for the first time.
    #[arg(long, default_value_t = false)]
    use_local_backend: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Cue {
        #[command(subcommand)]
        command: CueCommand,
    },

    Tf {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CueCommand {
    /// Evaluate CUE
    Eval {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Define CUE
    Def {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Export CUE
    Export {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

struct CuetContext<'a> {
    module_name: String,
    module_package: String,
    backend_override_value: Cow<'a, str>,
    cue_bin: PathBuf,
    tf_bin: PathBuf,
    target_dir: PathBuf,
    env: String,
    logger: logger::Logger,
}

impl CuetContext<'_> {
    fn output_dir(&self) -> PathBuf {
        self.target_dir.join(OUTPUT_FOLDER_NAME).join(&self.env)
    }

    fn handle_cue_command(&self, command: &CueCommand) -> Result<()> {
        let (cmd, args) = match command {
            CueCommand::Eval { args } => ("eval", args),
            CueCommand::Def { args } => ("def", args),
            CueCommand::Export { args } => ("export", args),
        };

        let expression = format!(
            EXPORT_EXPRESSION!(),
            self.module_name, self.backend_override_value, self.env
        );

        debug!(
            self.logger,
            "Executing cue {} with expression: {}", cmd, expression
        );

        let mut command = std::process::Command::new(&self.cue_bin);
        let package_input = format!(".:{}", self.module_package);
        command
            .current_dir(&self.target_dir)
            .arg(cmd)
            .arg(package_input)
            .arg("-e")
            .arg(expression)
            .args(args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        let status = run_command(&self.logger, &mut command)?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }

    fn handle_tf_command<T>(&self, args: &[T]) -> Result<()>
    where
        T: AsRef<OsStr> + std::fmt::Debug,
    {
        let output_dir = self.output_dir();

        std::fs::create_dir_all(&output_dir)
            .into_diagnostic()
            .map_err(|e| miette::miette!("Failed to create output directory: {}", e))?;

        let output_file = output_dir.join(OUTPUT_FILE_NAME);

        let expression = format!(
            EXPORT_EXPRESSION!(),
            self.module_name, self.backend_override_value, self.env
        );

        let mut command = std::process::Command::new(&self.cue_bin);
        let package_input = format!(".:{}", self.module_package);
        command
            .current_dir(&self.target_dir)
            .arg("export")
            .arg(package_input)
            .arg("-e")
            .arg(expression)
            .arg("-f")
            .arg("-o")
            .arg(
                output_file
                    .strip_prefix(&self.target_dir)
                    .into_diagnostic()?,
            )
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        let export_status = run_command(&self.logger, &mut command)?;

        if !export_status.success() {
            std::process::exit(export_status.code().unwrap_or(1));
        }

        let mut command = std::process::Command::new(&self.tf_bin);
        command
            .current_dir(&output_dir)
            .args(args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        let tf_status = run_command(&self.logger, &mut command)?;

        if !tf_status.success() {
            std::process::exit(tf_status.code().unwrap_or(1));
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let cue_bin = resolve_tool(
        &cli.cue_path
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CUE_BIN)),
    )?;
    let tf_bin = resolve_tool(&cli.tf_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TF_BIN)))?;

    let target_dir = match cli.path {
        Some(p) => p,
        None => std::env::current_dir().into_diagnostic()?,
    }
    .canonicalize()
    .into_diagnostic()?;

    if !target_dir.exists() {
        return Err(miette::miette!(
            "Target directory {:?} does not exist",
            target_dir
        ));
    }

    let root = find_root(&target_dir)?;

    let module_name = target_dir
        .strip_prefix(&root)
        .into_diagnostic()
        .map_err(|_| miette::miette!("Target file must be inside the cuet workspace"))?
        .to_string_lossy()
        .into_owned();

    let backend_override_value = if cli.use_local_backend {
        Cow::Owned(format!("\"{LOCAL_STATE_FILE_NAME}\""))
    } else {
        Cow::Borrowed("null")
    };

    let module_package = target_dir
        .file_name()
        .ok_or_else(|| miette::miette!("Could not infer module package from path"))?
        .to_string_lossy()
        .into_owned();

    let ctx = CuetContext {
        logger: logger::Logger::new(cli.verbose),
        module_name,
        module_package,
        backend_override_value,
        cue_bin,
        tf_bin,
        target_dir,
        env: cli.env,
    };

    debug!(
        ctx.logger,
        "Configuration:\n    \
            - Cue Bin: {:?}\n    \
            - Tf Bin: {:?}\n    \
            - Root: {:?}\n    \
            - Module Location: {:?}\n    \
            - Module Name: {}\n    \
            - Module Package: {}\n    \
            - Env: {}\n    \
            - Command: {:?}",
        ctx.cue_bin,
        ctx.tf_bin,
        root,
        ctx.target_dir,
        ctx.module_name,
        ctx.module_package,
        ctx.env,
        cli.command
    );

    match cli.command {
        Commands::Cue { command } => ctx.handle_cue_command(&command)?,
        Commands::Tf { args } => ctx.handle_tf_command(&args)?,
    }

    Ok(())
}

fn run_command(
    logger: &logger::Logger,
    cmd: &mut std::process::Command,
) -> Result<std::process::ExitStatus, miette::ErrReport> {
    let program = cmd.get_program().to_str().unwrap();
    let args = cmd.get_args().map(|arg| arg.to_str().unwrap());

    let cmd_string = shell_words::join(std::iter::once(program).chain(args));

    info!(
        logger,
        "From: {}\n   Running: {}",
        cmd.get_current_dir()
            .map_or(Cow::from("<None>"), |x| x.to_string_lossy()),
        cmd_string
    );
    cmd.status().into_diagnostic()
}

fn resolve_tool(path: &Path) -> Result<PathBuf> {
    which::which(path)
        .into_diagnostic()
        .map_err(|e| miette::miette!("Could not find tool '{path:?}': {e}"))
}

fn find_root(start_path: &std::path::Path) -> Result<PathBuf> {
    start_path
        .ancestors()
        .find(|path| path.join(".cuetroot.cue").exists())
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| miette::miette!("Could not find .cuetroot.cue in ancestors"))
}
