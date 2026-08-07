use clap::Parser;
use miette::Result;

#[macro_use]
mod logger;
mod app;
mod cli;
mod completions;
mod environment;
mod execution;
mod reconciliation;
#[cfg(test)]
mod test_directory;
mod workspace;

fn main() -> Result<()> {
    completions::complete();

    if let Some(status) = app::run(cli::Cli::parse())?
        && !status.success()
    {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
