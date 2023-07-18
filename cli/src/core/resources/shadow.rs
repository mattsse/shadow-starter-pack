use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Represents a shadow contract
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowContract {
    /// The file name of the shadow contract
    pub file_name: String,
    /// The name of the shadow contract
    pub contract_name: String,
    /// The address of the shadow contract
    pub address: String,
    /// The runtime bytecode of the shadow contract.
    /// This is the bytecode that is stored on the shadow fork.
    pub runtime_bytecode: String,
}

/// Defines the interface for interacting with a Shadow store
///
/// The Shadow resource is responsible for storing and retrieving shadow contracts
/// from the Shadow store.
///
/// The Shadow store may be a file system, a database, or a remote service.
#[async_trait]
pub trait ShadowResource {
    async fn get(&self, address: &str) -> Result<ShadowContract, Box<dyn std::error::Error>>;
    async fn list(&self) -> Result<Vec<ShadowContract>, Box<dyn std::error::Error>>;
    async fn upsert(
        &self,
        shadow_contract: ShadowContract,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn remove(&self, address: &str) -> Result<(), Box<dyn std::error::Error>>;
}
