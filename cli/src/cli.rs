use clap::{Parser, Subcommand, ValueHint};
use clap_complete::engine::ArgValueCompleter;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "cuet")]
#[command(about = "A CLI for interacting with CUE-based Terraform setups")]
#[command(version)]
pub struct Cli {
    /// Verbosity
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Module and optional environment to target. Defaults to the current module.
    #[arg(
        short = 't',
        long,
        value_name = "MODULE[:ENV]",
        add = ArgValueCompleter::new(crate::completions::target_candidates)
    )]
    pub target: Option<Target>,

    /// Exact cuet workspace root. Defaults to discovery from the current directory.
    #[arg(
        short = 'w',
        long,
        value_name = "PATH",
        value_hint = ValueHint::DirPath
    )]
    pub workspace: Option<PathBuf>,

    /// Path to the 'cue' binary
    #[arg(long, value_hint = ValueHint::ExecutablePath)]
    pub cue_path: Option<PathBuf>,

    /// Path to the 'tofu' (or terraform) binary
    #[arg(long, value_hint = ValueHint::ExecutablePath)]
    pub tf_path: Option<PathBuf>,

    /// Path to the 'tfmigrate' binary
    #[arg(long, value_hint = ValueHint::ExecutablePath)]
    pub tfmigrate_path: Option<PathBuf>,

    /// Maximum duration for `OpenTofu` operations. Remote-state reads default to 30s.
    #[arg(long, global = true, value_name = "DURATION", value_parser = humantime::parse_duration)]
    pub timeout: Option<Duration>,

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

        let module = ModuleTarget::from_cli_component(module);

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

impl ModuleTarget {
    pub fn from_cli_component(module: &str) -> Self {
        if module.is_empty() {
            Self::Relative(PathBuf::from("."))
        } else if let Some(path) = module.strip_prefix('/') {
            Self::WorkspaceRelative(PathBuf::from(path))
        } else {
            Self::Relative(PathBuf::from(module))
        }
    }
}

pub fn parse_env(value: impl Into<String>) -> Result<Env, String> {
    let value = value.into();
    validate_env(&value)?;
    Ok(value)
}

pub fn validate_env(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("environment must match [A-Za-z0-9_-]+".to_owned());
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Print version information
    Version,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

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

    /// Migrate state for the selected module environment
    Migrate {
        #[command(subcommand)]
        command: MigrationCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum MigrationCommand {
    /// Validate migration history and repository layout
    Check,
    /// Print migration details as JSON
    Inspect,
    /// Validate a migration without updating remote state
    Plan {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Apply a migration to remote state
    Apply {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

impl MigrationCommand {
    pub fn tfmigrate_command_and_args(&self) -> Option<(&'static str, &[String])> {
        match self {
            Self::Plan { args } => Some(("plan", args)),
            Self::Apply { args } => Some(("apply", args)),
            Self::Check | Self::Inspect => None,
        }
    }

    pub fn args(&self) -> &[String] {
        match self {
            Self::Plan { args } | Self::Apply { args } => args,
            Self::Check | Self::Inspect => &[],
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum ModulesCommand {
    /// List modules in the cuet workspace
    List,
    /// Check CUE exports for every populated module environment
    ///
    /// Discovers every module in the workspace and every environment present in its
    /// evaluated infra.in. For each environment, exports the final
    /// infra.out[ENV].terraform value with CUE.
    ///
    /// Without --drift, this only checks that the generated Terraform configuration
    /// can be evaluated and concretely exported. It does not run cue vet,
    /// OpenTofu/Terraform validate, init, or plan, and it does not write generated
    /// Terraform or state files.
    ///
    /// Pass --drift to additionally run an OpenTofu/Terraform plan for every
    /// populated environment and fail if any plan reports changes. Changes include
    /// both unapplied configuration and infrastructure changed outside Terraform.
    /// Drift checks may initialize generated working directories, download
    /// providers, refresh remote objects, and lock state.
    ///
    /// --target is not accepted because this command always checks the whole
    /// workspace. Failures are collected and reported after all discoverable
    /// environments have been checked.
    Check {
        /// Run a plan for every populated environment and fail if any plan reports changes
        #[arg(long)]
        drift: bool,
    },
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
    use super::{
        Cli, Commands, CueCommand, MigrationCommand, ModuleTarget, ModulesCommand, Target,
    };
    use clap::Parser;
    use clap::error::ErrorKind;
    use clap_complete::Shell;
    use std::path::PathBuf;
    use std::time::Duration;

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
    fn test_cli_parses_timeout() {
        let cli = Cli::try_parse_from(["cuet", "--timeout", "250ms", "tf", "output"]).unwrap();

        assert_eq!(cli.timeout, Some(Duration::from_millis(250)));
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
    fn test_cli_parses_modules_check() {
        let cli = Cli::try_parse_from(["cuet", "modules", "check"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Modules {
                command: ModulesCommand::Check { drift: false }
            }
        ));
    }

    #[test]
    fn test_cli_parses_modules_check_drift() {
        let cli = Cli::try_parse_from(["cuet", "modules", "check", "--drift"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Modules {
                command: ModulesCommand::Check { drift: true }
            }
        ));
    }

    #[test]
    fn test_cli_explains_modules_check() {
        let error = Cli::try_parse_from(["cuet", "modules", "check", "--help"]).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains("exports the final infra.out[ENV].terraform value with CUE"));
        assert!(help.contains("both unapplied configuration and infrastructure changed"));
        assert!(help.contains("--target is not accepted"));
        assert!(help.contains("--drift"));
    }

    #[test]
    fn test_cli_parses_version_command() {
        let cli = Cli::try_parse_from(["cuet", "version"]).unwrap();

        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn test_cli_displays_version_from_flags() {
        for flag in ["-V", "--version"] {
            let error = Cli::try_parse_from(["cuet", flag]).unwrap_err();

            assert_eq!(error.kind(), ErrorKind::DisplayVersion);
            assert_eq!(
                error.to_string(),
                format!("cuet {}\n", env!("CARGO_PKG_VERSION"))
            );
        }
    }

    #[test]
    fn test_cli_preserves_verbose_short_flag() {
        let cli = Cli::try_parse_from(["cuet", "-v", "modules", "list"]).unwrap();

        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parses_completion_shell() {
        let cli = Cli::try_parse_from(["cuet", "completions", "bash"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Completions { shell: Shell::Bash }
        ));
    }

    #[test]
    fn test_cli_rejects_unknown_completion_shell() {
        let error = Cli::try_parse_from(["cuet", "completions", "unknown"]).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn test_cli_parses_migrate_command() {
        let cli =
            Cli::try_parse_from(["cuet", "-t", "/infra/new:prod", "migrate", "plan"]).unwrap();

        let Commands::Migrate {
            command: MigrationCommand::Plan { args },
        } = cli.command
        else {
            panic!("expected migrate plan command");
        };
        assert!(args.is_empty());
        assert_eq!(
            cli.target,
            Some(Target {
                module: ModuleTarget::WorkspaceRelative(PathBuf::from("infra/new")),
                environment: Some("prod".to_owned()),
            })
        );
    }

    #[test]
    fn test_cli_parses_migration_automation_commands() {
        let check = Cli::try_parse_from(["cuet", "migrate", "check"]).unwrap();
        let inspect = Cli::try_parse_from(["cuet", "migrate", "inspect"]).unwrap();
        let apply = Cli::try_parse_from(["cuet", "migrate", "apply", "--config=ci.hcl"]).unwrap();

        assert!(matches!(
            check.command,
            Commands::Migrate {
                command: MigrationCommand::Check
            }
        ));
        assert!(matches!(
            inspect.command,
            Commands::Migrate {
                command: MigrationCommand::Inspect
            }
        ));
        assert!(matches!(
            apply.command,
            Commands::Migrate {
                command: MigrationCommand::Apply { args }
            }
            if args == ["--config=ci.hcl"]
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
