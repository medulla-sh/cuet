use clap::Parser;
use miette::Result;

#[macro_use]
mod logger;
mod app;
mod cli;
mod execution;
#[cfg(test)]
mod test_support;
mod workspace;

fn main() -> Result<()> {
    let status = app::run(cli::Cli::parse())?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
