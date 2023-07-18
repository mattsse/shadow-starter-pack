use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Interface for interacting with Etherscan.
/// The Etherscan resource is responsible for fetching data from Etherscan.
#[async_trait]
pub trait EtherscanResource {
    /// Fetch the contract creation metadata from Etherscan
    async fn get_contract_creation(
        &self,
        address: &str,
    ) -> Result<GetContractCreationResponse, reqwest::Error>;

    /// Fetch the source code from Etherscan
    async fn get_source_code(
        &self,
        contract_address: &str,
    ) -> Result<GetSourceCodeResponse, reqwest::Error>;
}

/// Represents the response from the Etherscan API for the contract creation endpoint
/// https://docs.etherscan.io/api-endpoints/contracts#get-contract-creator-and-creation-tx-hash
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractCreationResponse {
    pub status: String,
    pub message: String,
    pub result: Vec<ContractCreationResult>,
}

/// Represents a single result in the Etherscan API for the contract creation endpoint
/// https://docs.etherscan.io/api-endpoints/contracts#get-contract-creator-and-creation-tx-hash
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractCreationResult {
    pub contract_address: String,
    pub contract_creator: String,
    pub tx_hash: String,
}

/// Represents the response from the Etherscan API for the source code endpoint
/// https://docs.etherscan.io/api-endpoints/contracts#get-contract-source-code-for-verified-contract-source-codes
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSourceCodeResponse {
    pub status: String,
    pub message: String,
    pub result: Vec<SourceCodeResult>,
}

/// Represents a single result in the Etherscan API for the source code endpoint
/// https://docs.etherscan.io/api-endpoints/contracts#get-contract-source-code-for-verified-contract-source-codes
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceCodeResult {
    pub constructor_arguments: String,
}
