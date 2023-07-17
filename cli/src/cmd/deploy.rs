use std::fs;

use clap::Args;
use thiserror::Error;

use crate::resources::etherscan::ContractCreationResult;

#[derive(Args)]
pub struct Deploy {
    /// The shadow contract to deploy
    ///
    /// Can either be in the form ContractFile.sol (if the filename and contract name are the same), or ContractFile.sol:ContractName.
    pub contract: String,

    /// The address of the shadow contract to deploy
    pub address: String,
}

#[derive(Error, Debug)]
pub enum DeployError {
    /// Catch-all error
    #[error("Error: {0}")]
    DefaultError(String),
    /// Error related to reading the artifact file
    #[error("Error: {0}")]
    ArtifactIOError(#[from] std::io::Error),
    /// Error related to parsing the artifact file
    #[error("Error: {0}")]
    ArtifactParseError(#[from] serde_json::Error),
    /// Error related to Etherscan
    #[error("Etherscan API error: {0}")]
    EtherscanAPIError(#[source] reqwest::Error),
}

impl Deploy {
    pub async fn run(&self) -> Result<(), DeployError> {
        // Parse the contract string
        let (file_name, contract_name) = parse_contract_string(&self.contract);

        // Read the init bytecode from the artifact file
        let init_bytecode = self.get_init_bytecode(&file_name, &contract_name)?;

        // Fetch the contract creation metadata from Etherscan
        let contract_creation_metadata = self.fetch_contract_creation_metadata().await?;

        println!("Init bytecode: {:?}", init_bytecode);
        println!(
            "Contract creation metadata: {:?}",
            contract_creation_metadata
        );
        Ok(())
    }

    /// Returns the init bytecode of the shadow contract from the artifact file.
    fn get_init_bytecode(
        &self,
        file_name: &String,
        contract_name: &String,
    ) -> Result<alloy_primitives::Bytes, DeployError> {
        let contract = read_contract_object(file_name, contract_name)?;
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
        let etherscan = crate::resources::etherscan::Etherscan::new(String::from(env!(
            "ETHERSCAN_API_KEY",
            "Please set an ETHERSCAN_API_KEY"
        )));
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
}

/// Parses the contract string into a file name and contract name.
///
/// If the contract name is not provided, it is assumed to be the
/// same as the file name.
fn parse_contract_string(contract: &String) -> (String, String) {
    let mut parts = contract.splitn(2, ':');
    let file_name = parts.next().unwrap().to_owned();
    let contract_name = match parts.next() {
        Some(name) => name.to_owned(),
        None => {
            let mut parts = file_name.splitn(2, '.');
            let name = parts.next().unwrap().to_owned();
            name
        }
    };
    (file_name, contract_name)
}

/// Reads the contract object from the corresponding artifact file.
fn read_contract_object(
    file_name: &String,
    contract_name: &String,
) -> Result<alloy_json_abi::ContractObject, DeployError> {
    let file_path = format!("contracts/out/{}/{}.json", file_name, contract_name);
    let contents = fs::read_to_string(file_path).map_err(DeployError::ArtifactIOError)?;
    let contract: alloy_json_abi::ContractObject =
        serde_json::from_str(&contents).map_err(DeployError::ArtifactParseError)?;
    Ok(contract)
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
