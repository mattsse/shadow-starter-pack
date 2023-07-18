use std::env;

use clap::Args;

pub use crate::core::actions::fork::ForkError;
use crate::resources::shadow::LocalShadowStore;
use ethers::providers::{Http, Provider};

#[derive(Args)]
pub struct Fork {}

impl Fork {
    pub async fn run(&self) -> Result<(), ForkError> {
        let eth_rpc_url = env!("ETH_RPC_URL", "Please set an ETH_RPC_URL").to_owned();

        // Build the provider
        let provider =
            Provider::<Http>::try_from(&eth_rpc_url).expect("Please set a valid ETH_RPC_URL");

        // Build the resources
        let shadow_resource = LocalShadowStore::new(
            env::current_dir()
                .unwrap()
                .as_path()
                .to_str()
                .unwrap()
                .to_owned(),
        );

        // Build the action
        let fork =
            crate::core::actions::fork::Fork::new(provider, shadow_resource, eth_rpc_url).await?;

        // Run the action
        fork.run().await?;

        Ok(())
    }
}
