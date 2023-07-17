use serde::{Deserialize, Serialize};

/// Represents the resource for the Etherscan API client
pub struct Etherscan {
    api_key: String,
}

/// Represents the response from the Etherscan API for the contract creation endpoint
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContractCreationResponse {
    pub status: String,
    pub message: String,
    pub result: Vec<ContractCreationResult>,
}

/// Represents a single result in the Etherscan API for the contract creation endpoint
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractCreationResult {
    pub contract_address: String,
    pub contract_creator: String,
    pub tx_hash: String,
}

impl Etherscan {
    pub fn new(api_key: String) -> Self {
        Etherscan { api_key }
    }

    /// https://docs.etherscan.io/api-endpoints/contracts#get-contract-creator-and-creation-tx-hash
    pub async fn get_contract_creation(
        &self,
        address: &String,
    ) -> Result<GetContractCreationResponse, reqwest::Error> {
        let url = format!(
            "https://api.etherscan.io/api?module=contract&action=getcontractcreation&contractaddresses={}&apikey={}",
            address, self.api_key
        );
        let response = reqwest::get(&url)
            .await?
            .json::<GetContractCreationResponse>()
            .await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::Etherscan;

    #[tokio::test(flavor = "multi_thread")]
    async fn can_get_contract_creation() {
        let etherscan = Etherscan {
            api_key: String::from(env!("ETHERSCAN_API_KEY", "Please set an ETHERSCAN_API_KEY")),
        };
        let response = etherscan
            .get_contract_creation(&String::from("0x7a250d5630b4cf539739df2c5dacb4c659f2488d"))
            .await
            .unwrap();
        assert_eq!(response.status, String::from("1"));
        assert_eq!(response.message, String::from("OK"));
        assert_eq!(response.result.len(), 1);
        let result = response.result.get(0).unwrap();
        assert_eq!(
            result.contract_address,
            String::from("0x7a250d5630b4cf539739df2c5dacb4c659f2488d")
        );
        assert_eq!(
            result.contract_creator,
            String::from("0x9c33eacc2f50e39940d3afaf2c7b8246b681a374")
        );
        assert_eq!(
            result.tx_hash,
            String::from("0x4fc1580e7f66c58b7c26881cce0aab9c3509afe6e507527f30566fbf8039bcd0")
        );
    }
}
