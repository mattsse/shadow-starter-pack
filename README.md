<img src=".github/logo.png" alt="Shadow logo" align="left" width="120" />

# Shadow Hackathon
[Shadow](https://tryshadow.xyz) is a developer platform that
allows you to modify any contract to generate custom onchain data.

Using Shadow, you have the ability to add entirely new event
logs. These modifications are made directly within contract
source code, written in Solidity (or Vyper). You can then
deploy your shadow contract on a forked chain which “shadows”
mainnet transactions, where mainnet gas costs and contract
size limitations are removed.

See the [Shadow Technical Sheet](https://docs.google.com/document/d/1T_ra0tEO-j04yPvLlcqI4t2pJHjIJCa-BCY8lk1pISc/edit) for more details.

## Prerequisites
1. Have [Foundry](https://github.com/foundry-rs/foundry) installed
1. An Ethereum RPC URL (e.g. https://eth-mainnet.g.alchemy.com/v2/abcdefg0123456789)
1. A Shadow account (reach out to [@emilyhsia](https://github.com/emilyhsia) if you still
need to get set up with an account)

## Setup
1. Clone this repository
```
$ git clone git@github.com:shadow-hq/hackathon.git
```
2. Set environment variables
```
RPC_URL=<your_rpc_url>
SHADOW_PROJECT_ID=<your_project_id>
```

## Getting Started
This section will walk you through the steps to create,
test, and deploy a shadow contract. By the end of this
tutorial, you will:

1. Edit the Uniswap V2 Router contract to add a shadow event
called `Trade`.
1. Test that the `Trade` event gets emitted when a swap
occurs via Foundry unit tests.
1. Run a local shadow fork with the Uniswap V2 Router
shadow contract.
1. Deploy the Uniswap V2 Router shadow contract to your
hosted shadow fork.

### Steps
TODO