use alloy_primitives::{Bytes, U64};
use clap::Parser;
use std::str::FromStr;

use anvil::{
    cmd::NodeArgs,
    eth::{error::BlockchainError, EthApi},
    NodeHandle,
};
use anvil_core::eth::transaction::EthTransactionRequest;
use ethers::types::Transaction;
use ethers::{
    prelude::{Http, Provider},
    providers::Middleware,
};
use thiserror::Error;

use crate::resources::{
    artifacts::ArtifactsResource,
    etherscan::{ContractCreationResult, EtherscanResource},
};

const DEPLOYER_BALANCE: i64 = 1000000000000000000;
const DEPLOY_TX_GAS: i64 = 10000000;

/// Deploys a shadow contract to a shadow fork. Used by the `deploy` command.
pub struct Deploy<E: EtherscanResource, A: ArtifactsResource> {
    /// The name of the artifact file to use
    file_name: String,

    /// The name of the contract to deploy
    contract_name: String,

    /// The address of the shadow contract to deploy
    address: String,

    /// The Ethereum provider
    provider: Provider<Http>,

    /// The Artifacts resource
    artifacts_resource: A,

    /// The Etherscan resource
    etherscan_resource: E,
}

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum DeployError {
    /// Catch-all error
    #[error("DefaultError: {0}")]
    DefaultError(String),
    /// Blockchain error
    #[error("BlockchainError: {0}")]
    BlockchainError(#[from] BlockchainError),
    /// Error related to the artifacts store
    #[error("ArtifactError: {0}")]
    ArtifactError(#[from] Box<dyn std::error::Error>),
    /// Error related to Etherscan
    #[error("EtherscanError: {0}")]
    EtherscanError(#[source] reqwest::Error),
    /// Error related to the provider
    #[error("ProviderError: {0}")]
    ProviderError(#[from] ethers::providers::ProviderError),
}

impl<E: EtherscanResource, A: ArtifactsResource> Deploy<E, A> {
    pub fn new(
        file_name: String,
        contract_name: String,
        address: String,
        provider: Provider<Http>,
        artifacts_resource: A,
        etherscan_resource: E,
    ) -> Self {
        Deploy {
            file_name,
            contract_name,
            address,
            provider,
            artifacts_resource,
            etherscan_resource,
        }
    }

    pub async fn run(&self) -> Result<(), DeployError> {
        // Get the artifact bytecode
        let artifact_bytecode = self.get_artifact_bytecode()?;

        // Fetch the contract creation metadata from Etherscan
        let contract_creation_metadata = self.fetch_contract_creation_metadata().await?;

        // Fetch the constructor arguments from Etherscan
        let constructor_arguments = self.fetch_constructor_arguments().await?;

        // Fetch the contract creation transaction
        let contract_creation_transaction = self
            .fetch_contract_creation_transaction(&contract_creation_metadata.tx_hash)
            .await?;

        // Start a temporary fork to deploy the shadow contract
        let (api, anvil_handle) = self
            .start_anvil(
                contract_creation_transaction
                    .block_number
                    .map(|n| U64::from(n.as_u64())),
            )
            .await?;

        // Construct the init code
        let init_code = self
            .construct_init_code(&artifact_bytecode, &constructor_arguments)
            .await?;

        // Deploy the shadow contract and get the runtime bytecode
        let runtime_bytecode = self
            .get_runtime_bytecode(
                &api,
                &init_code,
                &contract_creation_metadata.contract_creator,
            )
            .await?;
        println!("Runtime bytecode: {:?}", runtime_bytecode);

        // Kill the fork
        anvil_handle.node_service.abort();

        Ok(())
    }

    /// Returns the init bytecode of the shadow contract from the artifact file.
    fn get_artifact_bytecode(&self) -> Result<Bytes, DeployError> {
        let contract: alloy_json_abi::ContractObject = self
            .artifacts_resource
            .get_artifact(&self.file_name, &self.contract_name)
            .map_err(DeployError::ArtifactError)?;
        match contract.bytecode {
            Some(bytecode) => Ok(bytecode),
            None => Err(DeployError::DefaultError(
                "Contract does not have bytecode".to_owned(),
            )),
        }
    }

    /// Fetches the contract creation metadata from Etherscan.
    async fn fetch_contract_creation_metadata(
        &self,
    ) -> Result<ContractCreationResult, DeployError> {
        // Fetch the contract creation metadata from Etherscan
        let response = self
            .etherscan_resource
            .get_contract_creation(&self.address)
            .await
            .map_err(DeployError::EtherscanError)?;

        // Check that the response is valid
        if response.status != "1" {
            return Err(DeployError::DefaultError(response.message));
        }

        // Check that the response contains exactly one result
        if response.result.len() != 1 {
            return Err(DeployError::DefaultError(
                "Expected exactly one result".to_owned(),
            ));
        }

        // Return the result
        let result = response.result.first().unwrap();
        Ok(result.clone())
    }

    /// Fetches the constructor arguments from Etherscan.
    async fn fetch_constructor_arguments(&self) -> Result<String, DeployError> {
        // Fetch the contract creation metadata from Etherscan
        let response = self
            .etherscan_resource
            .get_source_code(&self.address)
            .await
            .map_err(DeployError::EtherscanError)?;

        // Check that the response is valid
        if response.status != "1" {
            return Err(DeployError::DefaultError(response.message));
        }

        // Check that the response contains exactly one result
        if response.result.len() != 1 {
            return Err(DeployError::DefaultError(
                "Expected exactly one result".to_owned(),
            ));
        }

        // Return the result
        let result = response.result.first().unwrap();
        Ok(result.constructor_arguments.clone())
    }

    /// Fetches the contract creation transaction.
    async fn fetch_contract_creation_transaction(
        &self,
        tx_hash: &str,
    ) -> Result<Transaction, DeployError> {
        let response = self
            .provider
            .get_transaction(ethers::types::H256::from_str(tx_hash).unwrap())
            .await
            .map_err(DeployError::ProviderError)?;

        match response {
            Some(transaction) => Ok(transaction),
            None => Err(DeployError::DefaultError(
                "Transaction not found".to_owned(),
            )),
        }
    }

    /// Starts an anvil fork, which is used to deploy the shadow contract.
    async fn start_anvil(
        &self,
        block_number: Option<U64>,
    ) -> Result<(EthApi, NodeHandle), DeployError> {
        let anvil_args = anvil_args(
            self.provider.url().as_str(),
            block_number
                .map(|n| n.to_string())
                .unwrap_or_else(|| "latest".to_owned())
                .as_str(),
        );
        let (api, node_handle) = anvil::spawn(anvil_args.into_node_config()).await;
        Ok((api, node_handle))
    }

    /// Constructs the init code to create the shadow contract.
    async fn construct_init_code(
        &self,
        artifact_bytecode: &Bytes,
        constructor_arguments: &String,
    ) -> Result<Vec<u8>, DeployError> {
        let mut init_code = artifact_bytecode.to_vec();
        let mut constructor_arguments = hex::decode(constructor_arguments).unwrap();
        init_code.append(&mut constructor_arguments);
        Ok(init_code)
    }

    /// Deploys the shadow contract onto the anvil fork to get the runtime bytecode.
    async fn get_runtime_bytecode(
        &self,
        api: &EthApi,
        init_code: &[u8],
        deployer_address: &str,
    ) -> Result<String, DeployError> {
        // Insure the deployer has enough balance to deploy the shadow contract
        let deployer = ethers::types::H160::from_str(deployer_address).unwrap();
        api.anvil_set_balance(deployer, ethers::types::U256::from(DEPLOYER_BALANCE))
            .await
            .map_err(DeployError::BlockchainError)?;

        // Impersonate the deployer and send the deploy transaction
        api.anvil_impersonate_account(deployer)
            .await
            .map_err(DeployError::BlockchainError)?;
        let request = EthTransactionRequest {
            from: Some(deployer),
            value: Some(ethers::types::U256::from(0_i64)),
            gas: Some(ethers::types::U256::from(DEPLOY_TX_GAS)),
            data: Some(ethers::types::Bytes::from(init_code.to_owned())),
            ..Default::default()
        };
        let deploy_tx_hash = api
            .send_transaction(request)
            .await
            .map_err(DeployError::BlockchainError)?;

        // Mine the transaction
        api.evm_mine(None)
            .await
            .map_err(DeployError::BlockchainError)?;

        // Get the deployed contract address
        let deploy_tx_receipt = api
            .transaction_receipt(deploy_tx_hash)
            .await
            .map_err(DeployError::BlockchainError)?;
        let deployed_contract_address = match deploy_tx_receipt {
            Some(receipt) => match receipt.contract_address {
                Some(address) => address,
                None => {
                    return Err(DeployError::DefaultError(
                        "Failed to get contract address".to_owned(),
                    ))
                }
            },
            None => {
                return Err(DeployError::DefaultError(
                    "Failed to get transaction receipt".to_owned(),
                ))
            }
        };

        // Get the deployed contract code
        let code = api
            .get_code(deployed_contract_address, None)
            .await
            .map_err(DeployError::BlockchainError)?;
        Ok(hex::encode(code.as_ref()))
    }
}

fn anvil_args(eth_rpc_url: &str, block_number: &str) -> NodeArgs {
    NodeArgs::parse_from([
        "anvil",
        "--fork-url",
        eth_rpc_url,
        "--fork-block-number",
        block_number,
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
