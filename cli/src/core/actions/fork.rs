use anvil::{
    cmd::NodeArgs,
    eth::{error::BlockchainError, EthApi},
    NodeHandle,
};
use clap::Parser;
use ethers::{
    prelude::{providers::StreamExt, Provider},
    providers::{JsonRpcClient, Middleware, PubsubClient},
    types::{Transaction, TransactionReceipt},
};

use std::{collections::HashMap, str::FromStr};
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
    /// Provider error
    #[error("ProviderError: {0}")]
    ProviderError(#[from] ethers::providers::ProviderError),
    /// Blockchain error
    #[error("BlockchainError: {0}")]
    BlockchainError(#[from] BlockchainError),
}

impl<P: JsonRpcClient + PubsubClient> Fork<P> {
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

        // Start the block replay
        let mut stream = self.provider.subscribe_blocks().await?;
        while let Some(block) = stream.next().await {
            self.replay_block(&api, block.number.unwrap()).await?;
        }

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
                ethers::types::Bytes::from(
                    hex::decode(shadow_contract.runtime_bytecode.as_str()).unwrap(),
                ),
            )
            .await
            .map_err(|e| ForkError::DefaultError(e.to_string()))?;
        }

        Ok(())
    }

    async fn replay_block(
        &self,
        api: &EthApi,
        block_number: ethers::types::U64,
    ) -> Result<(), ForkError> {
        // Get the block with transactions
        let block = self
            .provider
            .get_block_with_txs(block_number)
            .await
            .map_err(ForkError::ProviderError)?;

        if let None = block {
            return Err(ForkError::DefaultError(format!(
                "Block {} not found",
                block_number
            )));
        }

        // Get the receipts
        let receipts = self
            .provider
            .get_block_receipts(block_number)
            .await
            .map_err(ForkError::ProviderError)?;
        // Build a map of transaction hashes to receipts
        let receipt_map = receipts
            .into_iter()
            .map(|receipt| (receipt.transaction_hash, receipt))
            .collect::<std::collections::HashMap<_, _>>();

        // Set up the block
        let block = block.unwrap();
        if let Some(base_fee) = block.base_fee_per_gas {
            api.anvil_set_next_block_base_fee_per_gas(base_fee)
                .await
                .map_err(ForkError::BlockchainError)?;
        }
        api.evm_set_next_block_timestamp(block.timestamp.as_u64())
            .map_err(ForkError::BlockchainError)?;

        // Send the transactions
        for tx in block.transactions {
            if self.should_replay(&tx, &receipt_map) {
                // Impersonate the sender and send the transaction
                api.anvil_set_balance(tx.from, ethers::types::U256::from("100000000000000000000"))
                    .await
                    .map_err(ForkError::BlockchainError)?;

                api.anvil_impersonate_account(tx.from)
                    .await
                    .map_err(ForkError::BlockchainError)?;
                api.send_raw_transaction(tx.rlp())
                    .await
                    .map_err(ForkError::BlockchainError)?;
            }
        }

        // Mine the block
        api.evm_mine(None)
            .await
            .map_err(ForkError::BlockchainError)?;

        Ok(())
    }

    fn should_replay(
        &self,
        tx: &Transaction,
        receipts: &HashMap<ethers::types::H256, TransactionReceipt>,
    ) -> bool {
        let is_shadowed = tx
            .to
            .map(|to| self.is_shadowed(format!("0x{}", hex::encode(to.as_bytes())).as_str()))
            .unwrap_or(false);

        let is_success = receipts
            .get(&tx.hash)
            .map(|receipt| {
                receipt
                    .status
                    .map(|status| status.as_u64() == 1)
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        is_shadowed && is_success
    }

    fn is_shadowed(&self, address: &str) -> bool {
        self.shadow_contracts.iter().any(|c| c.address == address)
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
        "--no-rate-limit",
        "--hardfork",
        "latest",
    ])
}
