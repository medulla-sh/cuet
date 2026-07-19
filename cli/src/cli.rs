use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "cuet")]
#[command(about = "A CLI for interacting with CUE-based Terraform setups")]
pub struct Cli {
    /// Verbosity
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Module and optional environment to target. Defaults to the current module.
    #[arg(short = 't', long, value_name = "MODULE[:ENV]")]
    pub target: Option<Target>,

    /// Exact cuet workspace root. Defaults to discovery from the current directory.
    #[arg(short = 'w', long, value_name = "PATH")]
    pub workspace: Option<PathBuf>,

    /// Path to the 'cue' binary
    #[arg(long)]
    pub cue_path: Option<PathBuf>,

    /// Path to the 'tofu' (or terraform) binary
    #[arg(long)]
    pub tf_path: Option<PathBuf>,

    /// If set, will override to use a local backend instead of the framework configured backend.
    /// This is useful when creating a backend for the first time.
    #[arg(long, default_value_t = false)]
    pub use_local_backend: bool,

    #[command(subcommand)]
    pub command: Commands,
}

pub type Env = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub module: ModuleTarget,
    pub environment: Option<Env>,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            module: ModuleTarget::Relative(PathBuf::from(".")),
            environment: None,
        }
    }
}

impl FromStr for Target {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (module, environment) = match value.split_once(':') {
            Some((module, environment)) => (module, Some(parse_env(environment)?)),
            None => (value, None),
        };

        let module = if module.is_empty() {
            ModuleTarget::Relative(PathBuf::from("."))
        } else if let Some(path) = module.strip_prefix('/') {
            ModuleTarget::WorkspaceRelative(PathBuf::from(path))
        } else {
            ModuleTarget::Relative(PathBuf::from(module))
        };

        Ok(Self {
            module,
            environment,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleTarget {
    Relative(PathBuf),
    WorkspaceRelative(PathBuf),
}

pub fn parse_env(value: &str) -> Result<Env, String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("environment must match [A-Za-z0-9_-]+".to_owned());
    }
    Ok(value.to_owned())
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage modules in the cuet workspace
    Modules {
        #[command(subcommand)]
        command: ModulesCommand,
    },

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
pub enum ModulesCommand {
    /// List modules in the cuet workspace
    List,
}

#[derive(Subcommand, Debug)]
pub enum CueCommand {
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

impl CueCommand {
    pub fn command_and_args(&self) -> (&'static str, &[String]) {
        match self {
            Self::Eval { args } => ("eval", args),
            Self::Def { args } => ("def", args),
            Self::Export { args } => ("export", args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, CueCommand, ModuleTarget, ModulesCommand, Target};
    use clap::Parser;
    use clap::error::ErrorKind;
    use std::path::PathBuf;

    #[test]
    fn test_cli_parses_cue_arguments() {
        let cli = Cli::try_parse_from([
            "cuet",
            "--target",
            "infra/neon:dev",
            "cue",
            "export",
            "--out",
            "yaml",
        ])
        .unwrap();

        assert_eq!(
            cli.target,
            Some(Target {
                module: ModuleTarget::Relative(PathBuf::from("infra/neon")),
                environment: Some("dev".to_owned()),
            })
        );
        let Commands::Cue {
            command: CueCommand::Export { args },
        } = cli.command
        else {
            panic!("expected cue export command");
        };
        assert_eq!(args, ["--out", "yaml"]);
    }

    #[test]
    fn test_cli_parses_terraform_arguments() {
        let cli = Cli::try_parse_from([
            "cuet",
            "-t",
            ":global",
            "tf",
            "plan",
            "-target=google_project.main",
        ])
        .unwrap();

        assert_eq!(
            cli.target,
            Some(Target {
                module: ModuleTarget::Relative(PathBuf::from(".")),
                environment: Some("global".to_owned()),
            })
        );
        let Commands::Tf { args } = cli.command else {
            panic!("expected tf command");
        };
        assert_eq!(args, ["plan", "-target=google_project.main"]);
    }

    #[test]
    fn test_cli_parses_workspace_relative_module() {
        let cli = Cli::try_parse_from([
            "cuet",
            "-w",
            "/repo",
            "-t",
            "/infra/neon:prod",
            "tf",
            "plan",
        ])
        .unwrap();

        assert_eq!(cli.workspace, Some(PathBuf::from("/repo")));
        assert_eq!(
            cli.target,
            Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("infra/neon")),
                environment: Some("prod".to_owned()),
            })
        );
    }

    #[test]
    fn test_cli_parses_module_without_environment() {
        let cli = Cli::try_parse_from(["cuet", "-t", "../neon", "cue", "export"]).unwrap();

        assert_eq!(
            cli.target,
            Some(Target {
                module: ModuleTarget::Relative(PathBuf::from("../neon")),
                environment: None,
            })
        );
    }

    #[test]
    fn test_cli_parses_modules_without_target() {
        let cli = Cli::try_parse_from(["cuet", "modules", "list"]).unwrap();

        assert!(cli.target.is_none());
        assert!(matches!(
            cli.command,
            Commands::Modules {
                command: ModulesCommand::List
            }
        ));
    }

    #[test]
    fn test_cli_requires_modules_subcommand() {
        let error = Cli::try_parse_from(["cuet", "modules"]).unwrap_err();

        assert_eq!(
            error.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn test_cli_rejects_empty_environment() {
        let error = Cli::try_parse_from(["cuet", "-t", "infra/neon:", "tf", "plan"]).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("environment must match [A-Za-z0-9_-]+")
        );
    }

    #[test]
    fn test_cli_allows_empty_module_component() {
        let cli = Cli::try_parse_from(["cuet", "-t", "/infra//neon:dev", "tf", "plan"]).unwrap();

        assert_eq!(
            cli.target,
            Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("infra//neon")),
                environment: Some("dev".to_owned()),
            })
        );
    }

    #[test]
    fn test_cli_rejects_environment_path() {
        let error = Cli::try_parse_from(["cuet", "-t", ":../../prod", "tf", "plan"]).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("environment must match [A-Za-z0-9_-]+")
        );
    }
}
