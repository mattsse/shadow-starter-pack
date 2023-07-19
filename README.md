# Shadow Hackathon

<img src=".github/logo.png" alt="Shadow logo" width="120" />

----

[Shadow](https://tryshadow.xyz) is a developer platform that
allows you to modify any contract to generate custom onchain data.

This repo contains everything you need to get started writing
your own shadow events:
1. Example shadow contracts with shadow events.
1. Example unit tests to test your shadow contracts.
1. A CLI tool that allows you to run a light shadow fork
locally, and deploy a shadow contract to your hosted shadow fork.

----

# Prerequisites
1. Have [Foundry](https://github.com/foundry-rs/foundry) installed
1. An Ethereum RPC URL (you'll need both an HTTP and Websockets endpoint)
(e.g. https://eth-mainnet.g.alchemy.com/v2/abcdefg0123456789)
1. An Etherscan API key (get a free one [here](https://docs.etherscan.io/getting-started/viewing-api-usage-statistics))
1. A Shadow account (reach out to [@emilyhsia](https://github.com/emilyhsia)
if you still need to get set up with an account)

# Setup
1. Clone this repository
```
$ git clone git@github.com:shadow-hq/hackathon.git
```
2. Set environment variables
```
ETH_RPC_URL=<http_rpc_url>
WS_RPC_URL=<ws_rpc_url>
ETHERSCAN_API_KEY=<etherscan_api_key>
SHADOW_PROJECT_ID=<your_project_id>
```

# Tutorial
*Note: Make sure you have completed the Setup above
before going through this section.*

This tutorial will walk you through the steps to create,
test, and deploy your first shadow contract -- the Uniswap V2
Router contract.

By the end of this tutorial, you will:

1. Edit the contract to add a shadow event.
1. Test that the shadow event is emitted via Foundry unit tests.
1. Run a local shadow fork with the shadow contract.
1. Deploy the shadow contract to your hosted shadow fork.

## Background
We will be adding a new `Trade` event to the Uniswap V2 Router
contract. The `Trade` event will represent a single top-level
trade, which may involve multiple underlying swaps between
liquidity pools.

This repository is initialized with everything you need for
this tutorial. It includes:
1. The original source code for the Uniswap V2
Router contract (in `contracts/src`)
1. A patch of shadow contract changes to apply (in `contracts/patches`)
2. Foundry unit tests that check that a `Trade` event is
emitted when a swap transaction occurs on the contract (in
`contracts/test`).

## Steps
*Note: All commands should be run from the root of the project repo, unless
stated otherwise.*

### 0. Run the Foundry tests
```bash
forge test -vvv
```

You should see an output that looks like:
```bash
Test result: FAILED. 0 passed; 1 failed; 0 skipped; finished in 816.73ms
Ran 1 test suites: 0 tests passed, 1 failed, 0 skipped (1 total tests)

Failing tests:
Encountered 1 failing test in contracts/test/UniswapV2Router02.t.sol:UniswapV2Router02Test
[FAIL. Reason: Expected an emit, but the call reverted instead. Ensure you're testing the happy path when using the `expectEmit` cheatcode.] testSwapExactTokensForTokens_emitsTrade() (gas: 3495931)
```

The Foundry tests are expected to fail because we haven't made any
shadow contract changes yet. The Foundry tests check that a `Trade`
event is emitted when a swap transaction occurs on the contract.
Since the original Uniswap V2 router contract does not have the `Trade`
event, the tests will fail.

### 1. Apply the `trade` patch
```bash
git apply contracts/patches/trade.patch
```

Applying the patch will add a `Trade` event to the Uniswap V2 Router
contract. The `Trade` event will be emitted once for every top-level
swap transaction. Check out the patch diff here:

https://github.com/shadow-hq/hackathon/blob/c920b32e1748f40d37d2d047b6034e661efbfc32/contracts/patches/trade.patch#L1-L133

### 2. Run the Foundry tests again
```bash
forge test -vvv
```

Now you should see that the Foundry tests are passing after
we added the `Trade` event.

```bash
Running 1 test for contracts/test/UniswapV2Router02.t.sol:UniswapV2Router02Test
[PASS] testSwapExactTokensForTokens_emitsTrade() (gas: 3773314)
Test result: ok. 1 passed; 0 failed; 0 skipped; finished in 933.97ms
Ran 1 test suites: 1 tests passed, 0 failed, 0 skipped (1 total tests)
```

### 3. Run a local shadow fork
Now that we've tested our shadow contract, we're going to deploy
the contract onto a locally running shadow fork.

First deploy the shadow contract onto the shadow fork:
```bash
shadow deploy UniswapV2Router02.sol:UniswapV2Router02 0x7a250d5630b4cf539739df2c5dacb4c659f2488d
```

Then start the local shadow fork:
```bash
shadow fork
```

After a few seconds, you should see logs that look like this, which
show that the shadow fork is receiving mainnet transactions.

```bash
    Transaction: 0x808611d79770cdf60cc40b88f7e89392baa0437ff410f05b34342a3950804e74
    Gas used: 157618

    Transaction: 0xd1b6867acf8d30b5de47579f8466821d8d5e1f0195c1269c0d2b2bb0444a0d2b
    Gas used: 180083

    Transaction: 0xd051b9ec828d500c0fbbb49a17bf810fd56089f9dc1ae860adc720e4764bbc27
    Gas used: 162034

    Transaction: 0x2f8a1ede7cc8e99e59101f1dc4cd81b60e1c2dc7facd3e89e0486a8e9c0eabfb
    Gas used: 152165

    Transaction: 0xd23994e04d0f3fef2c900933552dda459c913849875d159521d214c72ee4774c
    Gas used: 155304

    Transaction: 0xaf2cc90e2e0509b1c818d90c404c4a6a1c6594d4470fa322a82651f96e94d04f
    Gas used: 147028

    Block Number: 17723714
    Block Hash: 0x3956ecce5f133b53aedc34a46ffc028917b39b3277bd1f424ecf654b57908929
    Block Time: "Wed, 19 Jul 2023 00:35:47 +0000"
```