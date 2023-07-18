use std::fs;

/// Defines the interface for interacting with an artifact store.
/// The Artifacts resource is responsible for fetching data from the artifact store.
pub trait ArtifactsResource {
    /// Get the artifact for a given contract
    fn get_artifact(
        &self,
        file_name: &String,
        contract_name: &String,
    ) -> Result<alloy_json_abi::ContractObject, Box<dyn std::error::Error>>;
}

/// The implementation of the Artifacts resource.
pub struct Artifacts {
    path: String,
}

impl Artifacts {
    pub fn new(path: String) -> Self {
        Artifacts { path }
    }
}

impl ArtifactsResource for Artifacts {
    fn get_artifact(
        &self,
        file_name: &String,
        contract_name: &String,
    ) -> Result<alloy_json_abi::ContractObject, Box<dyn std::error::Error>> {
        let file_path = format!("{}/{}/{}.json", self.path, file_name, contract_name);
        let contents = fs::read_to_string(file_path)?;
        serde_json::from_str(&contents).map_err(|e| e.into())
    }
}
