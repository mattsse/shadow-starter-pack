mod cmd;
mod resources;
use std::fmt;

use clap::{Parser, Subcommand};
use thiserror::Error;

#[derive(Parser)]
#[command(author, version)]
#[command(
    about = "shadow - a CLI tool to generate custom onchain event data, for any smart contract on Ethereum mainnet",
    long_about = "shadow is a CLI tool to generate custom onchain event data, for any smart contract on Ethereum mainnet.

shadow provides commands to:

- Start a local shadow fork (light)
- Deploy a shadow contract onto your local shadow fork
- Deploy a shadow contract onto your hosted shadow fork"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploy a shadow contract
    Deploy(cmd::deploy::Deploy),
}

/// Represents an error that can occur while running the CLI tool
#[derive(Error, Debug)]
enum CliError {
    /// Error related to the deploy command
    DeployError(cmd::deploy::DeployError),
    /// Error that should never occur
    Never,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::DeployError(err) => write!(f, "Deploy error: {}", err),
            CliError::Never => write!(
                f,
                "This error should never occur, please file a bug report with help@tryshadow.xyz."
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
        None => Err(CliError::Never),
    }
}
