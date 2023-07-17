use clap::Parser;
use std::{fs, str::FromStr};

use anvil::{
    cmd::NodeArgs,
    eth::{error::BlockchainError, EthApi},
    NodeHandle,
};
use anvil_core::eth::transaction::EthTransactionRequest;
use clap::Args;
use ethers::types::Bytes;
use thiserror::Error;

use crate::resources::etherscan::{ContractCreationResult, Etherscan};

const DEPLOYER_BALANCE: i64 = 1000000000000000000;
const DEPLOY_TX_GAS: i64 = 10000000;

#[derive(Args)]
pub struct Deploy {
    /// The shadow contract to deploy
    ///
    /// Can either be in the form ContractFile.sol (if the filename and contract name are the same), or ContractFile.sol:ContractName.
    pub contract: String,

    /// The address of the shadow contract to deploy
    pub address: String,
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
    /// Error related to reading the artifact file
    #[error("ArtifactIOError: {0}")]
    ArtifactIOError(#[from] std::io::Error),
    /// Error related to parsing the artifact file
    #[error("ArtifactParseError: {0}")]
    ArtifactParseError(#[from] serde_json::Error),
    /// Error related to Etherscan
    #[error("EtherscanAPIError: {0}")]
    EtherscanAPIError(#[source] reqwest::Error),
    /// Error related to the Anvil node
    #[error("AnvilError: {0}")]
    AnvilError(#[from] Box<dyn std::error::Error>),
}

impl Deploy {
    pub async fn run(&self) -> Result<(), DeployError> {
        // Parse the contract string
        let (file_name, contract_name) = parse_contract_string(&self.contract);

        // Read the bytecode from the artifact file
        let artifact_bytecode = self.get_artifact_bytecode(&file_name, &contract_name)?;

        // Build the Etherscan resource
        let etherscan = Etherscan::new(String::from(env!(
            "ETHERSCAN_API_KEY",
            "Please set an ETHERSCAN_API_KEY"
        )));

        // Fetch the contract creation metadata from Etherscan
        let contract_creation_metadata = self.fetch_contract_creation_metadata(&etherscan).await?;

        // Fetch the constructor arguments from Etherscan
        let constructor_arguments = self.fetch_constructor_arguments(&etherscan).await?;

        // Start a temporary fork to deploy the shadow contract
        let (api, anvil_handle) = self.start_anvil().await?;

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
    fn get_artifact_bytecode(
        &self,
        file_name: &String,
        contract_name: &String,
    ) -> Result<alloy_primitives::Bytes, DeployError> {
        let file_path = format!("contracts/out/{}/{}.json", file_name, contract_name);
        let contents = fs::read_to_string(file_path).map_err(DeployError::ArtifactIOError)?;
        let contract: alloy_json_abi::ContractObject =
            serde_json::from_str(&contents).map_err(DeployError::ArtifactParseError)?;
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
        etherscan: &Etherscan,
    ) -> Result<ContractCreationResult, DeployError> {
        // Fetch the contract creation metadata from Etherscan
        let response = etherscan
            .get_contract_creation(&self.address)
            .await
            .map_err(DeployError::EtherscanAPIError)?;

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
    async fn fetch_constructor_arguments(
        &self,
        etherscan: &Etherscan,
    ) -> Result<String, DeployError> {
        // Fetch the contract creation metadata from Etherscan
        let response = etherscan
            .get_source_code(&self.address)
            .await
            .map_err(DeployError::EtherscanAPIError)?;

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

    /// Starts an anvil fork, which is used to deploy the shadow contract.
    async fn start_anvil(&self) -> Result<(EthApi, NodeHandle), DeployError> {
        let node_args = NodeArgs::parse_from([
            "anvil",
            "--fork-url",
            env!("ETH_RPC_URL", "Please set an ETH_RPC_URL"),
            "--fork-block-number",
            "10207857", // TODO: Make this dynamic
            "--code-size-limit",
            "73728",
            "--base-fee",
            "0",
            "--gas-price",
            "0",
            "--no-mining",
            "--disable-gas-limit",
            "--hardfork",
            "latest",
        ]);
        let (api, node_handle) = anvil::spawn(node_args.into_node_config()).await;
        Ok((api, node_handle))
    }

    /// Constructs the init code to create the shadow contract.
    async fn construct_init_code(
        &self,
        artifact_bytecode: &alloy_primitives::Bytes,
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
            .await?;

        // Impersonate the deployer and send the deploy transaction
        api.anvil_impersonate_account(deployer).await?;
        let request = EthTransactionRequest {
            from: Some(deployer),
            value: Some(ethers::types::U256::from(0_i64)),
            gas: Some(ethers::types::U256::from(DEPLOY_TX_GAS)),
            data: Some(Bytes::from(init_code.to_owned())),
            ..Default::default()
        };
        let deploy_tx_hash = api.send_transaction(request).await?;

        // Mine the transaction
        api.evm_mine(None).await?;

        // Get the deployed contract address
        let deploy_tx_receipt = api.transaction_receipt(deploy_tx_hash).await?;
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
        let code = api.get_code(deployed_contract_address, None).await?;
        Ok(hex::encode(code.as_ref()))
    }
}

/// Parses the contract string into a file name and contract name.
///
/// If the contract name is not provided, it is assumed to be the
/// same as the file name.
fn parse_contract_string(contract: &str) -> (String, String) {
    let mut parts = contract.splitn(2, ':');
    let file_name = parts.next().unwrap().to_owned();
    let contract_name = match parts.next() {
        Some(name) => name.to_owned(),
        None => {
            let mut parts = file_name.splitn(2, '.');

            parts.next().unwrap().to_owned()
        }
    };
    (file_name, contract_name)
}

#[cfg(test)]
mod tests {

    #[test]
    fn can_parse_contract_string() {
        let contract_string = String::from("UniswapV2Router02.sol:UniswapV2Router02");
        let (file_name, contract_name) = super::parse_contract_string(&contract_string);
        assert_eq!(file_name, String::from("UniswapV2Router02.sol"));
        assert_eq!(contract_name, String::from("UniswapV2Router02"));

        let contract_string = String::from("UniswapV2Router02.sol");
        let (file_name, contract_name) = super::parse_contract_string(&contract_string);
        assert_eq!(file_name, String::from("UniswapV2Router02.sol"));
        assert_eq!(contract_name, String::from("UniswapV2Router02"));
    }
}
