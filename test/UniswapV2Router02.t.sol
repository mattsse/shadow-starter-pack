// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.6.6;
pragma experimental ABIEncoderV2;

import "forge-std/Test.sol";
import "../src/UniswapV2Router02/contracts/UniswapV2Router02.sol";

contract UniswapV2Router02Test is Test {
	event Trade(
		string platformName,
		address contractAddress,
		address tokenInAddress,
		address tokenOutAddress,
		uint amountIn,
		uint amountOut,
		address senderAddress
	);

	function testSwapExactTokensForTokens_emitsTrade() public {
		// Replay transaction: 0x885f7c26504b7e8048b0296d85f754dfb2c6fd9924199fb7a66495500280318f
		IUniswapV2Router02 router = deployAtBlock(11508784);
		address msgSender = address(0x0F4ee9631f4be0a63756515141281A3E2B293Bbe);

		// Tokens and amounts
		address tokenIn = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
		address tokenOut = address(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
		uint256 amountIn = 118494640556767876660;
		uint256 amountOutMin = 72605940477;
		uint256 amountOutExpected = 72648946181;

		// Set up the VM
		vm.startPrank(msgSender);
		IERC20(tokenIn).approve(address(router), amountIn);

		// Construct arguments
		address[] memory path = new address[](2);
		path[0] = tokenIn;
		path[1] = tokenOut;

		// Expect the Trade event to be emitted
		vm.expectEmit(address(router));
		emit Trade("uniswap-v2", address(router), tokenIn, tokenOut, amountIn, amountOutExpected, msgSender);

		// Execute the swap
		router.swapExactTokensForTokens(amountIn, amountOutMin, path, msgSender, 1608713475);
	}

	function deployAtBlock(uint256 blockNumber) internal returns (IUniswapV2Router02 router) {
		// Set up the VM at the block
		vm.createSelectFork(vm.envString("ETH_RPC_URL"), blockNumber);

		// Build constructor arguments and bytecode
		address factory = address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
		address weth = address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
		bytes memory args = abi.encode(factory, weth);
		bytes memory bytecode = abi.encodePacked(vm.getCode("UniswapV2Router02.sol:UniswapV2Router02"), args);

		// Deploy
		address deployed;
		assembly {
			deployed := create(0, add(bytecode, 0x20), mload(bytecode))
		}
		router = IUniswapV2Router02(deployed);
	}
}
