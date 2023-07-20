use alloy_json_abi::Event;
use ethers::{
    prelude::{providers::StreamExt, Provider},
    providers::{JsonRpcClient, Middleware, ProviderError, PubsubClient},
    types::Filter,
};
use std::{str::FromStr, sync::Arc};
use thiserror::Error;

use crate::{
    core::resources::{
        artifacts::ArtifactsResource,
        shadow::{ShadowContract, ShadowResource},
    },
    decode,
};

/// Subscribes to events from a shadow contract on
/// a local fork.
///
/// This action is used by the `events` command.
pub struct Events<P: JsonRpcClient> {
    /// The Ethereum provider
    provider: Arc<Provider<P>>,

    /// The shadow contract to listen to events for.
    shadow_contract: ShadowContract,

    /// The event to listen to.
    event: Event,
}

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum EventsError {
    /// Catch-all error
    #[error("CustomError: {0}")]
    CustomError(String),
    /// Provider error
    #[error("ProviderError: {0}")]
    ProviderError(#[from] ProviderError),
    /// Decoder error
    #[error("DecoderError: {0}")]
    DecoderError(#[from] Box<dyn std::error::Error>),
}

impl<P: JsonRpcClient + PubsubClient> Events<P> {
    pub async fn new<A: ArtifactsResource, S: ShadowResource>(
        file_name: String,
        contract_name: String,
        event_signature: String,
        provider: Provider<P>,
        artifacts_resource: A,
        shadow_resource: S,
    ) -> Result<Self, EventsError> {
        let provider = Arc::new(provider);

        // Get shadow contract
        let shadow_contract = shadow_resource
            .get_by_name(&file_name, &contract_name)
            .await
            .map_err(|e| {
                EventsError::CustomError(format!("Error getting shadow contract: {}", e))
            })?;

        // Get the artifact
        let artifact = artifacts_resource
            .get_artifact(&file_name, &contract_name)
            .map_err(|e| EventsError::CustomError(format!("Error getting artifact: {}", e)))?;

        // Get the event
        let event = get_event(&event_signature, &artifact);

        match event {
            Some(event) => Ok(Self {
                provider,
                shadow_contract,
                event,
            }),
            None => Err(EventsError::CustomError(format!(
                "Event signature not found in contract's ABI: {}",
                event_signature
            ))),
        }
    }

    pub async fn run(&self) -> Result<(), EventsError> {
        // Build logs filter
        let logs_filter = self.build_logs_filter();

        // Subscribe to log
        let mut stream = self.provider.subscribe_logs(&logs_filter).await?;
        while let Some(log) = stream.next().await {
            let result = self.on_log(log);
            if let Err(e) = result {
                log::warn!("Error processing log: {}", e);
            }
        }

        Ok(())
    }

    fn build_logs_filter(&self) -> Filter {
        Filter {
            address: Some(ethers::types::ValueOrArray::Value(
                ethers::types::H160::from_str(self.shadow_contract.address.as_str()).unwrap(),
            )),
            topics: [
                Some(ethers::types::ValueOrArray::Value(Some(
                    ethers::types::H256::from_slice(self.event.selector().as_slice()),
                ))),
                None,
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn on_log(&self, log: ethers::types::Log) -> Result<(), EventsError> {
        let decoded = decode::decode_log(&log, &self.event)?;
        let pretty = colored_json::to_colored_json_auto(&decoded).map_err(|e| {
            EventsError::CustomError(format!("Error serializing decoded event to JSON: {}", e))
        })?;
        let tx_hash = format!("0x{}", hex::encode(log.transaction_hash.unwrap()));
        println!("=> Transaction: {}", tx_hash);
        println!("{}", pretty);
        Ok(())
    }
}

// Get the event from the contract's ABI
fn get_event(
    event_signature: &str,
    contract_object: &alloy_json_abi::ContractObject,
) -> Option<Event> {
    contract_object
        .abi
        .events
        .iter()
        .flat_map(|(_, events)| events)
        .find(|e| e.signature() == event_signature)
        .cloned()
}
