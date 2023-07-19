use async_trait::async_trait;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::core::resources::shadow::{ShadowContract, ShadowResource};

/// The Shadow resource implementation that uses the local file
/// system as the Shadow store.
///
/// The Shadow contracts are stored in a file called `shadow.json`.
pub struct LocalShadowStore {
    path: String,
}

impl LocalShadowStore {
    pub fn new(path: String) -> Self {
        LocalShadowStore { path }
    }

    fn read_from_file(&self) -> Result<Vec<ShadowContract>, Box<dyn std::error::Error>> {
        let file_path = format!("{}/shadow.json", self.path);

        // Create the shadow file if it doesn't exist
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file_path.clone())
        {
            file.write_all("[]".as_bytes())?;
        }

        let contents = fs::read_to_string(file_path)?;
        let contracts: Vec<ShadowContract> = serde_json::from_str(&contents)?;
        Ok(contracts)
    }

    fn write_to_file(
        &self,
        contracts: Vec<ShadowContract>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path: String = format!("{}/shadow.json", self.path);
        let contents = serde_json::to_string(&contracts)?;
        let mut file = File::create(file_path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }
}

#[async_trait]
impl ShadowResource for LocalShadowStore {
    async fn get_by_address(
        &self,
        address: &str,
    ) -> Result<ShadowContract, Box<dyn std::error::Error>> {
        let contracts = self.read_from_file()?;
        let contract = contracts
            .iter()
            .find(|contract| contract.address == address)
            .ok_or("Contract not found")?;
        Ok(contract.clone())
    }

    async fn get_by_name(
        &self,
        file_name: &str,
        contract_name: &str,
    ) -> Result<ShadowContract, Box<dyn std::error::Error>> {
        let contracts = self.read_from_file()?;
        let contract = contracts
            .iter()
            .find(|contract| {
                contract.file_name == file_name && contract.contract_name == contract_name
            })
            .ok_or("Contract not found")?;
        Ok(contract.clone())
    }

    async fn list(&self) -> Result<Vec<ShadowContract>, Box<dyn std::error::Error>> {
        let contracts = self.read_from_file()?;
        Ok(contracts)
    }

    async fn upsert(
        &self,
        shadow_contract: ShadowContract,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut contracts = self.read_from_file()?;
        let index = contracts
            .iter()
            .position(|contract| contract.address == shadow_contract.address);
        match index {
            Some(index) => {
                contracts[index] = shadow_contract;
            }
            None => {
                contracts.push(shadow_contract);
            }
        }
        self.write_to_file(contracts)?;
        Ok(())
    }

    async fn remove(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut contracts = self.read_from_file()?;
        let index = contracts
            .iter()
            .position(|contract| contract.address == address);
        match index {
            Some(index) => {
                contracts.remove(index);
            }
            None => {
                return Err("Contract not found".into());
            }
        }
        self.write_to_file(contracts)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::resources::shadow::{ShadowContract, ShadowResource};
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[tokio::test(flavor = "multi_thread")]
    async fn can_get_by_address() {
        let path = test_fixture!("resources", "");
        let shadow_store = super::LocalShadowStore::new(path);

        let contract = shadow_store
            .get_by_address("0x7a250d5630b4cf539739df2c5dacb4c659f2488d")
            .await
            .unwrap();
        assert_eq!(contract.file_name, "UniswapV2Router02.sol");
        assert_eq!(contract.contract_name, "UniswapV2Router02");
        assert_eq!(
            contract.address,
            "0x7a250d5630b4cf539739df2c5dacb4c659f2488d"
        );
        assert_eq!(
            contract.runtime_bytecode,
            "UniswapV2Router02_dummyruntimebytecode"
        );

        let contract = shadow_store
            .get_by_address("0xef1c6e67703c7bd7107eed8303fbe6ec2554bf6b")
            .await
            .unwrap();
        assert_eq!(contract.file_name, "UniversalRouter.sol");
        assert_eq!(contract.contract_name, "UniversalRouter");
        assert_eq!(
            contract.address,
            "0xef1c6e67703c7bd7107eed8303fbe6ec2554bf6b"
        );
        assert_eq!(
            contract.runtime_bytecode,
            "UniversalRouter_dummyruntimebytecode"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_get_by_name() {
        let path = test_fixture!("resources", "");
        let shadow_store = super::LocalShadowStore::new(path);

        let contract = shadow_store
            .get_by_name("UniswapV2Router02.sol", "UniswapV2Router02")
            .await
            .unwrap();
        assert_eq!(contract.file_name, "UniswapV2Router02.sol");
        assert_eq!(contract.contract_name, "UniswapV2Router02");
        assert_eq!(
            contract.address,
            "0x7a250d5630b4cf539739df2c5dacb4c659f2488d"
        );
        assert_eq!(
            contract.runtime_bytecode,
            "UniswapV2Router02_dummyruntimebytecode"
        );

        let contract = shadow_store
            .get_by_name("UniversalRouter.sol", "UniversalRouter")
            .await
            .unwrap();
        assert_eq!(contract.file_name, "UniversalRouter.sol");
        assert_eq!(contract.contract_name, "UniversalRouter");
        assert_eq!(
            contract.address,
            "0xef1c6e67703c7bd7107eed8303fbe6ec2554bf6b"
        );
        assert_eq!(
            contract.runtime_bytecode,
            "UniversalRouter_dummyruntimebytecode"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_list() {
        let path = test_fixture!("resources", "");
        let shadow_store = super::LocalShadowStore::new(path);

        let contracts = shadow_store.list().await.unwrap();
        assert_eq!(contracts.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_insert() {
        // Create a temp directory with a shadow.json file
        let temp_dir = tempdir().unwrap();
        let file_path_buf = temp_dir.path().join("shadow.json");
        let file_path = file_path_buf.as_path();
        File::create(file_path).unwrap();
        fs::copy(test_fixture!("resources", "shadow.json"), file_path).unwrap();

        // Create a shadow store
        let shadow_store =
            super::LocalShadowStore::new(temp_dir.path().to_str().unwrap().to_string());

        // Insert a new contract
        let contract = ShadowContract {
            file_name: "Seaport.sol".to_string(),
            contract_name: "Seaport".to_string(),
            address: "0x00000000000001ad428e4906ae43d8f9852d0dd6".to_string(),
            runtime_bytecode: "Seaport_dummyruntimebytecode".to_string(),
        };
        shadow_store.upsert(contract.clone()).await.unwrap();

        // Check that the contract was inserted
        let contracts = shadow_store.list().await.unwrap();
        assert_eq!(contracts.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_update() {
        // Create a temp directory with a shadow.json file
        let temp_dir = tempdir().unwrap();
        let file_path_buf = temp_dir.path().join("shadow.json");
        let file_path = file_path_buf.as_path();
        File::create(file_path).unwrap();
        fs::copy(test_fixture!("resources", "shadow.json"), file_path).unwrap();

        // Create a shadow store
        let shadow_store =
            super::LocalShadowStore::new(temp_dir.path().to_str().unwrap().to_string());

        // Update a contract
        let contract = ShadowContract {
            file_name: "UniswapV2Router02.sol".to_string(),
            contract_name: "UniswapV2Router02".to_string(),
            address: "0x7a250d5630b4cf539739df2c5dacb4c659f2488d".to_string(),
            runtime_bytecode: "UniswapV2Router02_dummyruntimebytecode_new".to_string(),
        };
        shadow_store.upsert(contract.clone()).await.unwrap();

        // Check that the contract was updated
        let contracts = shadow_store.list().await.unwrap();
        assert_eq!(contracts.len(), 2);
        let contract = shadow_store
            .get_by_address("0x7a250d5630b4cf539739df2c5dacb4c659f2488d")
            .await
            .unwrap();
        assert_eq!(contract.file_name, "UniswapV2Router02.sol");
        assert_eq!(contract.contract_name, "UniswapV2Router02");
        assert_eq!(
            contract.address,
            "0x7a250d5630b4cf539739df2c5dacb4c659f2488d"
        );
        assert_eq!(
            contract.runtime_bytecode,
            "UniswapV2Router02_dummyruntimebytecode_new"
        );
    }
}
