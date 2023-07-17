mod cmd;
mod resources;
use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Deploy(deploy)) => {
            deploy.run();
        }
        None => {}
    }
}
