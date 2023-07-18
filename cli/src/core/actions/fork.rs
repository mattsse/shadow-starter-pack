use anvil::{cmd::NodeArgs, eth::EthApi, NodeHandle};
use clap::Parser;
use ethers::{prelude::Provider, providers::JsonRpcClient};
use std::str::FromStr;
use thiserror::Error;

use crate::core::resources::shadow::{ShadowContract, ShadowResource};
pub struct Fork<P: JsonRpcClient> {
    /// The Ethereum provider
    pub provider: Provider<P>,

    pub shadow_contracts: Vec<ShadowContract>,

    /// The RPC URL to use for the anvil fork
    pub eth_rpc_url: String,
}

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum ForkError {
    /// Catch-all error
    #[error("DefaultError: {0}")]
    DefaultError(String),
}

impl<P: JsonRpcClient> Fork<P> {
    pub async fn new<S: ShadowResource>(
        provider: Provider<P>,
        shadow_resource: S,
        eth_rpc_url: String,
    ) -> Result<Self, ForkError> {
        let shadow_contracts = shadow_resource
            .list()
            .await
            .map_err(|e| ForkError::DefaultError(e.to_string()))?;

        Ok(Self {
            provider,
            shadow_contracts,
            eth_rpc_url,
        })
    }
    pub async fn run(&self) -> Result<(), ForkError> {
        // Start the anvil fork
        let (api, _) = self.start_anvil().await?;

        // Override the shadow contracts
        self.override_contracts(&api).await?;

        Ok(())
    }

    /// Starts an anvil fork, which is used to deploy the shadow contract.
    async fn start_anvil(&self) -> Result<(EthApi, NodeHandle), ForkError> {
        let anvil_args = anvil_args(self.eth_rpc_url.as_str());
        let (api, node_handle) = anvil::spawn(anvil_args.into_node_config()).await;
        Ok((api, node_handle))
    }

    async fn override_contracts(&self, api: &EthApi) -> Result<(), ForkError> {
        // Override the contracts
        for shadow_contract in &self.shadow_contracts {
            api.anvil_set_code(
                ethers::types::H160::from_str(shadow_contract.address.as_str()).unwrap(),
                ethers::types::Bytes::from(shadow_contract.runtime_bytecode.as_bytes().to_owned()),
            )
            .await
            .map_err(|e| ForkError::DefaultError(e.to_string()))?;
        }

        Ok(())
    }
}

fn anvil_args(eth_rpc_url: &str) -> NodeArgs {
    NodeArgs::parse_from([
        "anvil",
        "--fork-url",
        eth_rpc_url,
        "--code-size-limit",
        usize::MAX.to_string().as_str(),
        "--base-fee",
        "0",
        "--gas-price",
        "0",
        "--no-mining",
        "--disable-gas-limit",
        "--hardfork",
        "latest",
    ])
}
