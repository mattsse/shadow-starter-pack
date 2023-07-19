use std::env;

use clap::Args;

pub use crate::core::actions::deploy::DeployError;
use crate::resources::{
    artifacts::LocalArtifactStore, etherscan::Etherscan, shadow::LocalShadowStore,
};
use ethers::providers::{Http, Provider};

#[derive(Args)]
pub struct Deploy {
    /// The shadow contract to deploy
    ///
    /// Can either be in the form ContractFile.sol (if the filename and contract name are the same), or ContractFile.sol:ContractName.
    pub contract: String,

    /// The address of the shadow contract to deploy
    pub address: String,
}

impl Deploy {
    pub async fn run(&self) -> Result<(), DeployError> {
        let http_rpc_url = env!("ETH_RPC_URL", "Please set an ETH_RPC_URL").to_owned();

        // Parse the contract string
        let (file_name, contract_name) = parse_contract_string(&self.contract);

        // Build the provider
        let provider =
            Provider::<Http>::try_from(&http_rpc_url).expect("Please set a valid ETH_RPC_URL");

        // Build the resources
        let artifacts_resource = LocalArtifactStore::new("contracts/out".to_owned());
        let etherscan_resource = Etherscan::new(String::from(env!(
            "ETHERSCAN_API_KEY",
            "Please set an ETHERSCAN_API_KEY"
        )));
        let shadow_resource = LocalShadowStore::new(
            env::current_dir()
                .unwrap()
                .as_path()
                .to_str()
                .unwrap()
                .to_owned(),
        );

        let deploy = crate::core::actions::deploy::Deploy {
            file_name,
            contract_name,
            address: self.address.clone(),
            provider,
            artifacts_resource,
            etherscan_resource,
            shadow_resource,
            http_rpc_url,
        };

        deploy.run().await?;

        Ok(())
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
