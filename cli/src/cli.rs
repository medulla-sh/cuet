use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cuet")]
#[command(about = "A CLI for interacting with CUE-based Terraform setups")]
pub struct Cli {
    /// Verbosity
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Path to the infra module to evaluate. Defaults to current directory.
    #[arg(short = 'p', long)]
    pub path: Option<PathBuf>,

    /// Path to the 'cue' binary
    #[arg(long)]
    pub cue_path: Option<PathBuf>,

    /// Path to the 'tofu' (or terraform) binary
    #[arg(long)]
    pub tf_path: Option<PathBuf>,

    /// Environment name
    #[arg(required = true)]
    pub env: String,

    /// If set, will override to use a local backend instead of the framework configured backend.
    /// This is useful when creating a backend for the first time.
    #[arg(long, default_value_t = false)]
    pub use_local_backend: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    use super::{Cli, Commands, CueCommand};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_cli_parses_cue_arguments() {
        let cli = Cli::try_parse_from([
            "cuet",
            "--path",
            "infra/neon",
            "dev",
            "cue",
            "export",
            "--out",
            "yaml",
        ])
        .unwrap();

        assert_eq!(cli.path, Some(PathBuf::from("infra/neon")));
        assert_eq!(cli.env, "dev");
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
            "global",
            "tf",
            "plan",
            "-target=google_project.main",
        ])
        .unwrap();

        assert_eq!(cli.env, "global");
        let Commands::Tf { args } = cli.command else {
            panic!("expected tf command");
        };
        assert_eq!(args, ["plan", "-target=google_project.main"]);
    }
}
