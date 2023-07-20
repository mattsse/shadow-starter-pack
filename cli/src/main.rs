mod cmd;
mod core;
mod decode;
#[macro_use]
mod macros;
mod resources;
use std::fmt;

use clap::{Parser, Subcommand};
use thiserror::Error;

#[derive(Parser)]
#[command(author, version)]
#[command(about = "Shadow any smart contract on Ethereum mainnet")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploy a shadow contract
    Deploy(cmd::deploy::Deploy),
    /// Start a local shadow fork
    Fork(cmd::fork::Fork),
    /// Listen to events from a shadow contract
    Events(cmd::events::Events),
}

/// Represents an error that can occur while running the CLI tool
#[derive(Error, Debug)]
enum CliError {
    /// Error related to the deploy command
    DeployError(cmd::deploy::DeployError),
    /// Error related to the fork command
    ForkError(cmd::fork::ForkError),
    /// Error related to the events command
    EventsError(cmd::events::EventsError),
    /// Error that should never occur
    Never,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::DeployError(err) => write!(f, "Deploy error: {}", err),
            CliError::ForkError(err) => write!(f, "Fork error: {}", err),
            CliError::EventsError(err) => write!(f, "Events error: {}", err),
            CliError::Never => write!(
                f,
                "This error should never occur, please file a bug report to help@tryshadow.xyz."
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Deploy(deploy)) => {
            deploy.run().await.map_err(CliError::DeployError)?;
            Ok(())
        }
        Some(Commands::Fork(fork)) => {
            fork.run().await.map_err(CliError::ForkError)?;
            Ok(())
        }
        Some(Commands::Events(events)) => {
            events.run().await.map_err(CliError::EventsError)?;
            Ok(())
        }
        None => Err(CliError::Never),
    }
}
