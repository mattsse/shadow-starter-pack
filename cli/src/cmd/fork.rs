use std::env;

use clap::Args;

pub use crate::core::actions::fork::ForkError;
use crate::resources::shadow::LocalShadowStore;
use ethers::providers::{Provider, Ws};

#[derive(Args)]
pub struct Fork {
    /// Whether to replay all transactions from mainnet. Defaults to false.
    ///
    /// Note: We only recommend using this flag if you have a way
    /// to run your shadow fork against a high-performance RPC url
    /// (i.e. running it on the same machine as your node). Otherwise,
    /// the block processing will be very slow (3-4 minutes per
    /// block), and you'll quickly run out of RPC compute units.
    #[clap(short, long)]
    pub all_txs: Option<bool>,
}

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
        let fork = crate::core::actions::fork::Fork::new(
            provider,
            shadow_resource,
            http_rpc_url,
            self.all_txs.unwrap_or(false),
        )
        .await?;

        // Run the action
        fork.run().await?;

        Ok(())
    }
}
