/// The interface for interacting with a store of artifacts.
/// The Artifacts resource is responsible for fetching data from an artifacts store.
pub trait ArtifactsResource {
    /// Get the artifact for a given contract
    fn get_artifact(
        &self,
        file_name: &str,
        contract_name: &str,
    ) -> Result<alloy_json_abi::ContractObject, Box<dyn std::error::Error>>;
}
