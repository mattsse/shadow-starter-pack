use std::env;

use clap::Args;

pub use crate::core::actions::fork::ForkError;
use crate::resources::shadow::LocalShadowStore;
use ethers::providers::{Provider, Ws};

#[derive(Args)]
pub struct Fork {}

impl Fork {
    pub async fn run(&self) -> Result<(), ForkError> {
        let http_rpc_url = env!("ETH_RPC_URL", "Please set an ETH_RPC_URL").to_owned();

        // Build the provider
        let provider =
            Provider::<Ws>::connect(env!("WS_RPC_URL", "Please set an WS_RPC_URL").to_owned())
                .await
                .map_err(ForkError::ProviderError)?;

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
            crate::core::actions::fork::Fork::new(provider, shadow_resource, http_rpc_url).await?;

        // Run the action
        fork.run().await?;

        Ok(())
    }
}
