use std::fs;

use clap::Args;

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
    pub fn run(&self) {
        let (file_name, contract_name) = parse_contract_string(&self.contract);
        let init_bytecode = self.get_init_bytecode(&file_name, &contract_name);
        init_bytecode.map(|bytecode| {
            println!("Init bytecode: {:?}", bytecode);
        });
    }

    /// Returns the init bytecode of the shadow contract from the artifact file.
    fn get_init_bytecode(
        &self,
        file_name: &String,
        contract_name: &String,
    ) -> Option<alloy_primitives::Bytes> {
        let contract = read_contract_object(file_name, contract_name);
        contract.bytecode
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
) -> alloy_json_abi::ContractObject {
    let file_path = format!("contracts/out/{}/{}.json", file_name, contract_name);
    let contents = fs::read_to_string(file_path).expect("Couldn't find or load the file.");
    let contract: alloy_json_abi::ContractObject =
        serde_json::from_str(&contents).expect("Couldn't parse the contract file.");
    contract
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
