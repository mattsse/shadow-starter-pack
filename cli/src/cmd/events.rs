use std::env;

use clap::Args;

pub use crate::core::actions::events::EventsError;
use crate::resources::{artifacts::LocalArtifactStore, shadow::LocalShadowStore};
use ethers::providers::{Provider, Ws};

use super::deploy::parse_contract_string;

#[derive(Args)]
pub struct Events {
    /// The shadow contract to listen to events for.
    ///
    /// Can either be in the form ContractFile.sol (if the filename and contract name are the same), or ContractFile.sol:ContractName.
    pub contract: String,

    /// The event signature to listen to.
    pub event_signature: String,
}

/// Listens to events from a shadow contract on a local fork.
///
/// The command uses the [`crate::core::actions::Events`] action
/// under the hood, using the local file-based artifact store,
/// and the local file-based shadow store.
impl Events {
    pub async fn run(&self) -> Result<(), EventsError> {
        // Parse the contract string
        let (file_name, contract_name) = parse_contract_string(&self.contract);

        // Build the provider
        let provider = Provider::<Ws>::connect("ws://localhost:8545".to_owned())
            .await
            .map_err(EventsError::ProviderError)?;

        // Build the resources
        let artifacts_resource = LocalArtifactStore::new("contracts/out".to_owned());
        let shadow_resource = LocalShadowStore::new(
            env::current_dir()
                .unwrap()
                .as_path()
                .to_str()
                .unwrap()
                .to_owned(),
        );

        // Build the action
        let events = crate::core::actions::Events::new(
            file_name,
            contract_name,
            self.event_signature.to_owned(),
            provider,
            artifacts_resource,
            shadow_resource,
        )
        .await?;

        // Run the action
        events.run().await?;

        Ok(())
    }
}
