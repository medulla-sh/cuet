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
mod terraform;
#[cfg(test)]
mod test_directory;
mod workspace;

fn main() -> Result<()> {
    completions::complete();

    let cli = cli::Cli::parse();
    if let Some(status) = app::run(&cli)?
        && !status.success()
    {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
