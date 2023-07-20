use anvil::{
    cmd::NodeArgs,
    eth::{error::BlockchainError, EthApi},
    NodeHandle,
};
use clap::Parser;
use ethers::{
    prelude::{providers::StreamExt, Provider},
    providers::{JsonRpcClient, Middleware, ProviderError, PubsubClient},
    types::{Transaction, TransactionReceipt},
};
use tokio::task::JoinSet;

use std::{collections::HashMap, str::FromStr, sync::Arc};
use thiserror::Error;

use crate::core::resources::shadow::{ShadowContract, ShadowResource};

/// Starts a local shadow fork using Anvil.
///
/// This action is used by the `fork` command.
///
/// To reduce latency, and to save on RPC compute units,
/// this local shadow fork will NOT replay all transactions
/// from mainnet. It will only replay the transactions that
/// were sent to shadowed contracts.
///
/// This means that the local shadow fork state will not be
/// identical to mainnet, but it will be close enough for
/// demonstration purposes.
///
/// We're using Anvil's EVM for this local shadow fork, which
/// does not have gas limit bypassing enabled. This means that
/// the gas used by the shadow contracts will be different from
/// the gas used on mainnet.
pub struct Fork<P: JsonRpcClient + 'static> {
    /// The Ethereum provider
    pub provider: Arc<Provider<P>>,

    // The shadow contracts to use on the fork
    pub shadow_contracts: Vec<ShadowContract>,

    /// The HTTP RPC URL to use for the anvil fork
    pub http_rpc_url: String,

    /// Whether to replay all transactions from mainnet
    pub all_txs: bool,
}

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum ForkError {
    /// Catch-all error
    #[error("CustomError: {0}")]
    CustomError(String),
    /// Provider error
    #[error("ProviderError: {0}")]
    ProviderError(#[from] ProviderError),
    /// Blockchain error
    #[error("BlockchainError: {0}")]
    BlockchainError(#[from] BlockchainError),
}

impl<P: JsonRpcClient + PubsubClient> Fork<P> {
    pub async fn new<S: ShadowResource>(
        provider: Provider<P>,
        shadow_resource: S,
        http_rpc_url: String,
        all_txs: bool,
    ) -> Result<Self, ForkError> {
        let provider = Arc::new(provider);
        let shadow_contracts = shadow_resource
            .list()
            .await
            .map_err(|e| ForkError::CustomError(e.to_string()))?;

        Ok(Self {
            provider,
            shadow_contracts,
            http_rpc_url,
            all_txs,
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
            let result = self.replay_block(&api, block.number.unwrap());
            if let Err(e) = result.await {
                log::warn!("Error replaying block: {}", e);
            }
        }

        Ok(())
    }

    /// Starts an anvil fork, which is used as a local shadow fork.
    async fn start_anvil(&self) -> Result<(EthApi, NodeHandle), ForkError> {
        let anvil_args = anvil_args(self.http_rpc_url.as_str());
        let (api, node_handle) = anvil::spawn(anvil_args.into_node_config()).await;
        Ok((api, node_handle))
    }

    /// Overrides the shadow contract bytecode on the anvil fork.
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
            .map_err(|e| ForkError::CustomError(e.to_string()))?;
        }

        Ok(())
    }

    /// Replays a block on the anvil fork.
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

        if block.is_none() {
            return Err(ForkError::CustomError(format!(
                "Block {} not found",
                block_number
            )));
        }

        // Fetch the receipts
        let block = block.unwrap();
        let receipts = self.fetch_receipts(&block.transactions).await?;

        // Set up the block
        if let Some(base_fee) = block.base_fee_per_gas {
            api.anvil_set_next_block_base_fee_per_gas(base_fee)
                .await
                .map_err(ForkError::BlockchainError)?;
        }
        api.evm_set_next_block_timestamp(block.timestamp.as_u64())
            .map_err(ForkError::BlockchainError)?;

        // Send the transactions
        for tx in block.transactions {
            if self.should_replay(&tx, &receipts) {
                // Give the wallet extra ETH for the transaction before sending it
                api.anvil_set_balance(tx.from, ethers::types::U256::from("100000000000000000000"))
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

    /// Fetches the receipts for a list of transactions in parallel
    async fn fetch_receipts(
        &self,
        transactions: &[Transaction],
    ) -> Result<HashMap<ethers::types::H256, TransactionReceipt>, ForkError> {
        let mut receipt_map = HashMap::new();

        let mut join_set = JoinSet::new();

        // Spawn a task for each transaction receipt fetch
        for tx in transactions.iter() {
            let tx_hash = tx.hash;
            let provider = self.provider.clone();
            join_set.spawn(async move {
                let receipt = provider.get_transaction_receipt(tx_hash).await?;
                Ok::<Option<TransactionReceipt>, ProviderError>(receipt)
            });
        }

        while let Some(result) = join_set.join_next().await {
            let receipt = result
                .map_err(|e| ForkError::CustomError(e.to_string()))?
                .map_err(|e| {
                    ForkError::CustomError(format!("Error getting transaction receipt: {}", e))
                })?;

            match receipt {
                Some(receipt) => {
                    receipt_map.insert(receipt.transaction_hash, receipt);
                }
                None => {
                    return Err(ForkError::CustomError("Receipt not found.".to_string()));
                }
            }
        }

        Ok(receipt_map)
    }

    fn should_replay(
        &self,
        tx: &Transaction,
        receipts: &HashMap<ethers::types::H256, TransactionReceipt>,
    ) -> bool {
        if self.all_txs {
            return true;
        }

        // If the transaction is not to a shadowed contract, don't replay it
        let is_shadowed = tx
            .to
            .map(|to| self.is_shadowed(format!("0x{}", hex::encode(to.as_bytes())).as_str()))
            .unwrap_or(false);

        // If the transaction is not successful, don't replay it
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

fn anvil_args(http_rpc_url: &str) -> NodeArgs {
    NodeArgs::parse_from([
        "anvil",
        "--fork-url",
        http_rpc_url,
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
