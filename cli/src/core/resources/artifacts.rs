/// Defines the interface for interacting with an Artifacts store.
///
/// The Artifacts resource is responsible for retrieving artifacts from
/// an artifacts store.
///
/// The Artifacts store may be a file system, a database, or a remote service.
pub trait ArtifactsResource {
    /// Get the artifact for a given contract
    fn get_artifact(
        &self,
        file_name: &str,
        contract_name: &str,
    ) -> Result<alloy_json_abi::ContractObject, Box<dyn std::error::Error>>;
}
