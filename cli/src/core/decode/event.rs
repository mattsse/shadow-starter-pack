use alloy_json_abi::{Event, Param};
use serde::{Serialize, Serializer};
use serde_json::Value;

use super::param::ToEthAbiParamType;
use super::token::Token;

/// Decodes a log using the given event ABI.
///
/// Returns a JSON object with the parameter names as
/// keys and the decoded topics and data as values.
///
/// Example:
/// {
///     "from": "0x73ede13ab9c28bc4302e94c1d1e7f755988a9158",
///     "to": "0x91364516d3cad16e1666261dbdbb39c881dbe9ee",
///     "value": "69000000000000000000"
/// }
pub fn decode_log(
    log: &ethers::types::Log,
    event: &Event,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Decode the topics
    let mut topics = decode_topics(log, event)?;

    // Decode the data
    let data = decode_data(log, event)?;

    // Merge the topics and data
    merge(&mut topics, data);

    Ok(topics)
}

/// Decodes the log topics using the given event ABI.
///
/// Returns a JSON object with the parameter names as
/// keys and the decoded topics as values.
fn decode_topics(
    log: &ethers::types::Log,
    event: &Event,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Get the indexed parameters
    let indexed_params = event
        .inputs
        .iter()
        .filter(|input| input.indexed)
        .map(|p| p.to_owned())
        .collect::<Vec<_>>();

    // Build the ethabi types
    let mut ethabi_types = Vec::new();
    for param in indexed_params.iter() {
        ethabi_types.push(param.to_eth_abi_param_type()?);
    }

    // Combine the topic bytes
    let topics = log
        .topics
        .iter()
        .skip(1)
        .flat_map(|t| t.as_bytes())
        .map(|b| b.to_owned())
        .collect::<Vec<_>>();

    // Decode the topics
    let tokens = ethabi::decode_whole(&ethabi_types, &topics)?;

    // Build the map
    let mut map = serde_json::Map::new();
    for (i, event_param) in indexed_params.iter().enumerate() {
        let param = Param {
            name: event_param.name.clone(),
            ty: event_param.ty.clone(),
            internal_type: event_param.internal_type.clone(),
            components: event_param.components.clone(),
        };
        let token = Token::new(tokens[i].clone());
        let param_and_token = ParamAndValue {
            param,
            value: token,
        };
        map.insert(event_param.name.clone(), param_and_token.to_value());
    }

    // Create the value
    let value = serde_json::to_value(map)?;

    Ok(value)
}

/// Decodes log data using the given event ABI.
///
/// Returns a JSON object with the parameter names as
/// keys and the decoded data as values.
fn decode_data(
    log: &ethers::types::Log,
    event: &Event,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Get the non-indexed parameters
    let non_indexed_params = event
        .inputs
        .iter()
        .filter(|input| !input.indexed)
        .map(|p| p.to_owned())
        .collect::<Vec<_>>();

    // Build the ethabi types
    let mut eth_abi_types = Vec::new();
    for param in non_indexed_params.iter() {
        eth_abi_types.push(param.to_eth_abi_param_type()?);
    }

    // Decode the data
    let tokens = ethabi::decode(&eth_abi_types, &log.data)?;

    // Build the token map
    let mut map = serde_json::Map::new();
    for (i, event_param) in non_indexed_params.iter().enumerate() {
        let param = Param {
            name: event_param.name.clone(),
            ty: event_param.ty.clone(),
            internal_type: event_param.internal_type.clone(),
            components: event_param.components.clone(),
        };
        let token = Token::new(tokens[i].clone());
        let param_and_token = ParamAndValue {
            param,
            value: token,
        };
        map.insert(event_param.name.clone(), param_and_token.to_value());
    }

    // Create the value
    let value = serde_json::to_value(map)?;

    Ok(value)
}

fn merge(a: &mut Value, b: Value) {
    match (a, b) {
        (a @ &mut Value::Object(_), Value::Object(b)) => {
            let a = a.as_object_mut().unwrap();
            for (k, v) in b {
                merge(a.entry(k).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b,
    }
}

struct ParamAndValue {
    pub param: Param,
    pub value: Token,
}

impl ParamAndValue {
    pub fn to_value(&self) -> serde_json::Value {
        if self.param.is_complex_type() {
            // Get the components of the complex type
            let param_components = self.param.components.clone();

            // We have an array of complex values (e.g. Swap[])
            //
            // To handle an array of complex values, we need to
            // iterate over the array and decode each value.
            //
            // In the case of an array, the underlying value is an
            // array of complex values (e.g. [(string, address, uint256), (string, address, uint256)]).
            // We need to iterate over each of those complex values
            // and map the parameter names with the decoded fields.
            //
            // We do this by creating a new [`ParamAndValue`] for each
            // item in the array (which all share the same complex param type).
            // Then we call `to_value()` on each of those [`ParamAndValue`]s.
            //
            // Example:
            //  param_components = Array(Tuple(string, address, uint256))
            //  nested_values = Token(Array([("abc", "0x0000", 1), ("def", "0x0000", 2)]))
            if let ethabi::Token::Array(values) = self.value.underlying() {
                let array_values = values
                    .iter()
                    .map(|t| {
                        let param_and_value = ParamAndValue {
                            param: self.param.clone(),
                            value: Token::new(t.clone()),
                        };
                        param_and_value.to_value()
                    })
                    .collect::<Vec<_>>();
                return serde_json::to_value(&array_values).unwrap();
            }

            // We have a complex type (e.g. Swap)
            //
            // To handle a complex type, we need to map the parameter names
            // with the decoded values.
            //
            // Example:
            //  param_components = Tuple(string, address, uint256)
            //  nested_values = Token("abc", "0x0000", 1)
            let nested_values = self.value.clone().into_tokens();
            let param_and_values = param_components
                .iter()
                .zip(nested_values.iter())
                .map(|(param, token)| ParamAndValue {
                    param: param.clone(),
                    value: Token::new(token.clone()),
                })
                .fold(serde_json::Map::new(), |mut acc, param_and_token| {
                    acc.insert(
                        param_and_token.param.name.clone(),
                        param_and_token.to_value(),
                    );
                    acc
                });
            serde_json::to_value(&param_and_values).unwrap()
        } else {
            // If we have an array of simple values (e.g. uint256[]),
            // convert the array of values to an array of strings.
            if let ethabi::Token::Array(tokens) = self.value.underlying() {
                let array_values = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
                return serde_json::to_value(array_values).unwrap();
            }

            // Otherwise, just convert the value to a string.
            serde_json::to_value(self.value.to_string()).unwrap()
        }
    }
}

impl Serialize for ParamAndValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_value().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use ethers::{
        providers::{Http, Middleware, Provider},
        types::Log,
    };
    use serde_json::json;
    use std::str::FromStr;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn can_decode_log() {
        // Simple
        let log = erc20_transfer_log().await.unwrap();
        let event = erc20_transfer_event();
        let expected = json!(
            {
                "from": "0x73ede13ab9c28bc4302e94c1d1e7f755988a9158",
                "to": "0x91364516d3cad16e1666261dbdbb39c881dbe9ee",
                "value": "69000000000000000000"
            }
        );
        let actual = decode_log(&log, &event).unwrap();
        assert_eq!(expected, actual);

        // Nested
        let log = seaport_order_fulfilled_log().await.unwrap();
        let event = seaport_order_fulfilled_event();
        let expected = json!(
            {
                "offerer": "0xab9fcb219f0706a468485d3d41029a843a6df05d",
                "zone": "0xf49c52948bb9b0764b495978da0b21941c63380b",
                "orderHash": "0f996c590324cc7b8ecf2c5d908ec8915549b4847267b2f4a141356605e6c71c",
                "recipient": "0x0a082a17087305756eb9bfc5cd87506e3cfaac33",
                "offer": [{
                    "itemType": "2",
                    "token": "0x8c3c0274c33f263f0a55d129cfc8eaa3667a9e8b",
                    "identifier": "15967959419969011",
                    "amount": "1",
                }],
                "consideration": [
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "45600000000000000",
                      "recipient": "0xab9fcb219f0706a468485d3d41029a843a6df05d"
                    },
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "960000000000000",
                      "recipient": "0x74ce08242c97fac3be8b63a9d5061c5ef2c1c3a8"
                    },
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "480000000000000",
                      "recipient": "0x31c388503566d2e0ba335d22792bddf90bc86c82"
                    },
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "960000000000000",
                      "recipient": "0xca9337244b5f04cb946391bc8b8a980e988f9a6a"
                    }
                ]
            }
        );
        let actual = decode_log(&log, &event).unwrap();
        assert_eq!(expected, actual);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_decode_data() {
        // Simple
        let log = erc20_transfer_log().await.unwrap();
        let event = erc20_transfer_event();
        let expected = json!(
            {
                "value": "69000000000000000000"
            }
        );
        let actual = decode_data(&log, &event).unwrap();
        assert_eq!(expected, actual);

        // Nested
        let log = seaport_order_fulfilled_log().await.unwrap();
        let event = seaport_order_fulfilled_event();
        let expected = json!(
            {
                "orderHash": "0f996c590324cc7b8ecf2c5d908ec8915549b4847267b2f4a141356605e6c71c",
                "recipient": "0x0a082a17087305756eb9bfc5cd87506e3cfaac33",
                "offer": [{
                    "itemType": "2",
                    "token": "0x8c3c0274c33f263f0a55d129cfc8eaa3667a9e8b",
                    "identifier": "15967959419969011",
                    "amount": "1",
                }],
                "consideration": [
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "45600000000000000",
                      "recipient": "0xab9fcb219f0706a468485d3d41029a843a6df05d"
                    },
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "960000000000000",
                      "recipient": "0x74ce08242c97fac3be8b63a9d5061c5ef2c1c3a8"
                    },
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "480000000000000",
                      "recipient": "0x31c388503566d2e0ba335d22792bddf90bc86c82"
                    },
                    {
                      "itemType": "0",
                      "token": "0x0000000000000000000000000000000000000000",
                      "identifier": "0",
                      "amount": "960000000000000",
                      "recipient": "0xca9337244b5f04cb946391bc8b8a980e988f9a6a"
                    }
                  ]
            }
        );
        let actual = decode_data(&log, &event).unwrap();
        assert_eq!(expected, actual);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_decode_topics() {
        // Simple
        let log = erc20_transfer_log().await.unwrap();
        let event = erc20_transfer_event();

        let expected = json!(
            {
                "from": "0x73ede13ab9c28bc4302e94c1d1e7f755988a9158",
                "to": "0x91364516d3cad16e1666261dbdbb39c881dbe9ee"
            }
        );
        let actual = decode_topics(&log, &event).unwrap();
        assert_eq!(expected, actual);

        // Nested
        let log = seaport_order_fulfilled_log().await.unwrap();
        let event = seaport_order_fulfilled_event();

        let expected = json!(
            {
                "offerer": "0xab9fcb219f0706a468485d3d41029a843a6df05d",
                "zone": "0xf49c52948bb9b0764b495978da0b21941c63380b"
            }
        );
        let actual = decode_topics(&log, &event).unwrap();
        assert_eq!(expected, actual);
    }

    async fn erc20_transfer_log() -> Result<Log, Box<dyn std::error::Error>> {
        // Build the provider
        let http_rpc_url = env!("ETH_RPC_URL", "Please set an ETH_RPC_URL").to_owned();
        let provider =
            Provider::<Http>::try_from(&http_rpc_url).expect("Please set a valid ETH_RPC_URL");

        let receipt = provider
            .get_transaction_receipt(
                ethers::types::H256::from_str(
                    "0x52356815ed88ccbd6c38b42bacd706d0f8c21839fa30e858e364869d3dffc049",
                )
                .unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        let log = receipt.logs[0].clone();
        Ok(log)
    }

    fn erc20_transfer_event() -> Event {
        let s = r#"{
            "name": "Transfer",
            "type": "event",
            "inputs": [
                {
                    "name": "from",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "to",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "value",
                    "type": "uint256",
                    "indexed": false,
                    "internalType": "uint256"
                }
            ],
            "anonymous": false
        }"#;
        serde_json::from_str(s).unwrap()
    }

    async fn seaport_order_fulfilled_log() -> Result<Log, Box<dyn std::error::Error>> {
        // Build the provider
        let http_rpc_url = env!("ETH_RPC_URL", "Please set an ETH_RPC_URL").to_owned();
        let provider =
            Provider::<Http>::try_from(&http_rpc_url).expect("Please set a valid ETH_RPC_URL");

        let receipt = provider
            .get_transaction_receipt(
                ethers::types::H256::from_str(
                    "0xcfcd490c4ec2f5ff9063794746760b4e9fa3991c7cd5044d3a4f5bf50b156b34",
                )
                .unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        let log = receipt.logs[0].clone();
        Ok(log)
    }

    fn seaport_order_fulfilled_event() -> Event {
        let s = r#"{
            "name": "OrderFulfilled",
            "type": "event",
            "inputs": [
                {
                    "name": "orderHash",
                    "type": "bytes32",
                    "indexed": false,
                    "internalType": "bytes32"
                },
                {
                    "name": "offerer",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "zone",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "recipient",
                    "type": "address",
                    "indexed": false,
                    "internalType": "address"
                },
                {
                    "name": "offer",
                    "type": "tuple[]",
                    "indexed": false,
                    "components": [
                        {
                            "name": "itemType",
                            "type": "uint8",
                            "internalType": "enumItemType"
                        },
                        {
                            "name": "token",
                            "type": "address",
                            "internalType": "address"
                        },
                        {
                            "name": "identifier",
                            "type": "uint256",
                            "internalType": "uint256"
                        },
                        {
                            "name": "amount",
                            "type": "uint256",
                            "internalType": "uint256"
                        }
                    ],
                    "internalType": "structSpentItem[]"
                },
                {
                    "name": "consideration",
                    "type": "tuple[]",
                    "indexed": false,
                    "components": [
                        {
                            "name": "itemType",
                            "type": "uint8",
                            "internalType": "enumItemType"
                        },
                        {
                            "name": "token",
                            "type": "address",
                            "internalType": "address"
                        },
                        {
                            "name": "identifier",
                            "type": "uint256",
                            "internalType": "uint256"
                        },
                        {
                            "name": "amount",
                            "type": "uint256",
                            "internalType": "uint256"
                        },
                        {
                            "name": "recipient",
                            "type": "address",
                            "internalType": "addresspayable"
                        }
                    ],
                    "internalType": "structReceivedItem[]"
                }
            ],
            "anonymous": false
        }"#;
        serde_json::from_str(s).unwrap()
    }
}
