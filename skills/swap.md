---
name: swap
description: "Swap ERC20 tokens on Base using 0x DEX aggregator via quoter.defirelay.com"
version: 5.7.0
author: starkbot
homepage: https://0x.org
metadata: {"requires_auth": false, "clawdbot":{"emoji":"🔄"}}
tags: [crypto, defi, swap, dex, base, trading, 0x]
 requires_tools: [web, x, token_lookup, register_set, decode_calldata, web3_function_call, x402_fetch, x402_rpc, list_queued_web3_tx, broadcast_web3_tx]  
---

# Token Swap Integration (0x via DeFi Relay)

## CRITICAL: Two Things That Will Cause Reverts

### 1. ETH Must Be Wrapped First!
When selling ETH, you MUST wrap it to WETH first. The swap always uses WETH, not native ETH.

### 2. ALLOWANCE MUST BE SET Before Swapping!
**This is the #1 cause of failed swaps.** Before executing any swap, you MUST:
1. Check the current allowance for the sell token
2. If allowance is insufficient, approve the swap contract (Permit2: `0x000000000022D473030F116dDEE9F6B43aC78BA3`)
3. Only then execute the swap

**WETH is especially prone to this issue** because after wrapping ETH, the freshly minted WETH has zero allowance!

---

## Workflow A: Swapping ETH → Token

Use this when the user wants to swap ETH for another token.

### 1. Lookup WETH as sell token
```tool:token_lookup
symbol: WETH
cache_as: sell_token
```

### 2. Check ETH and WETH balances

Check WETH balance:
```tool:web3_function_call
preset: weth_balance
network: base
call_only: true
```

Check ETH balance:
```tool:x402_rpc
preset: get_balance
network: base
```

**Important:** Report both balances to the user. If they have enough WETH already, skip steps 3-4 and go directly to step 5.

### 3. Set wrap amount (only if WETH balance is insufficient)
```tool:register_set
key: wrap_amount
value: "100000000000000"
```

### 4. Wrap ETH to WETH (only if WETH balance is insufficient)
```tool:web3_function_call
preset: weth_deposit
network: base
```

### 5. Check WETH Allowance for Permit2 (CRITICAL!)
**After wrapping, freshly minted WETH has ZERO allowance. You MUST check and approve!**

Set the token address for allowance check:
```tool:register_set
key: token_address
value: "0x4200000000000000000000000000000000000006"
```

Set the spender to Permit2 contract:
```tool:register_set
key: spender_address
value: "0x000000000022D473030F116dDEE9F6B43aC78BA3"
```

Check current allowance:
```tool:web3_function_call
preset: erc20_allowance
network: base
call_only: true
```

### 6. Approve WETH if Allowance is Insufficient
**If allowance is less than the swap amount, you MUST approve first!**

Set a large approval amount (max uint256 for unlimited):
```tool:register_set
key: approve_amount
value: "115792089237316195423570985008687907853269984665640564039457584007913129639935"
```

Approve Permit2 to spend WETH:
```tool:web3_function_call
preset: erc20_approve
network: base
```

**Wait for the approval transaction to confirm before proceeding!**

### 7. Lookup buy token
```tool:token_lookup
symbol: USDC
cache_as: buy_token
```

### 8. Set sell amount
```tool:register_set
key: sell_amount
value: "100000000000000"
```

### 9. Get swap quote
```tool:x402_fetch
preset: swap_quote
network: base
cache_as: swap_quote
```

**NOTE:** The tool automatically retries up to 3 times on failure. **If the quote still fails after retries, STOP IMMEDIATELY.** Do not proceed with the swap. Inform the user: "Unable to get a swap quote for this trade. The swap cannot be completed." Common reasons include: unsupported token pair, amount too small, insufficient liquidity, or service unavailable.

### 10. Decode swap calldata (REQUIRED - DO NOT SKIP!)
**⚠️ YOU MUST RUN THIS STEP BEFORE EXECUTING THE SWAP. Do NOT pass raw calldata to web3_function_call. The preset reads from registers that this tool sets.**

Decode the raw calldata from the swap quote into function parameters:
```tool:decode_calldata
abi: 0x_settler
calldata_register: swap_quote
cache_as: swap
```

This sets registers: `swap_function`, `swap_contract`, `swap_value`, `swap_param_0` through `swap_param_4`

### 11. Execute swap
**Only call this AFTER decode_calldata has set the registers above. Do NOT pass a `params` array - the preset reads from registers.**
```tool:web3_function_call
preset: swap_execute
network: base
```

### 12. Verify queued transaction
After queueing, confirm the transaction details:
```tool:list_queued_web3_tx
status: pending
limit: 1
```

### 13. Broadcast when ready
```tool:broadcast_web3_tx
```

---

## Workflow B: Swapping Token → Token or Eth

Use this when selling any token OTHER than ETH (e.g., USDC → WETH).

### 1. Lookup sell token and check balance

Lookup the sell token:
```tool:token_lookup
symbol: USDC
cache_as: sell_token
```

Check the sell token balance (use the erc20_balance preset after setting token_address):
```tool:register_set
key: token_address
value: "<sell_token_address from lookup>"
```

```tool:web3_function_call
preset: erc20_balance
network: base
call_only: true
```

**Important:** Report the balance to the user. If insufficient, stop and inform them.

### 2. Check Allowance for Permit2 (CRITICAL!)
**Before swapping any ERC20 token, you MUST check its allowance!**

The token_address should already be set from step 1. Set the spender to Permit2:
```tool:register_set
key: spender_address
value: "0x000000000022D473030F116dDEE9F6B43aC78BA3"
```

Check current allowance:
```tool:web3_function_call
preset: erc20_allowance
network: base
call_only: true
```

### 3. Approve Token if Allowance is Insufficient
**If allowance is less than the swap amount, you MUST approve first!**

Set a large approval amount (max uint256 for unlimited):
```tool:register_set
key: approve_amount
value: "115792089237316195423570985008687907853269984665640564039457584007913129639935"
```

Approve Permit2 to spend the token:
```tool:web3_function_call
preset: erc20_approve
network: base
```

**Wait for the approval transaction to confirm before proceeding!**

### 4. Lookup buy token
```tool:token_lookup
symbol: WETH
cache_as: buy_token
```

### 5. Set sell amount
```tool:register_set
key: sell_amount
value: "1000000"
```

### 6. Get swap quote
```tool:x402_fetch
preset: swap_quote
network: base
cache_as: swap_quote
```

**NOTE:** The tool automatically retries up to 3 times on failure. **If the quote still fails after retries, STOP IMMEDIATELY.** Do not proceed with the swap. Inform the user: "Unable to get a swap quote for this trade. The swap cannot be completed." Common reasons include: unsupported token pair, amount too small, insufficient liquidity, or service unavailable.

### 7. Decode swap calldata (REQUIRED - DO NOT SKIP!)
**⚠️ YOU MUST RUN THIS STEP BEFORE EXECUTING THE SWAP. Do NOT pass raw calldata to web3_function_call. The preset reads from registers that this tool sets.**

Decode the raw calldata from the swap quote into function parameters:
```tool:decode_calldata
abi: 0x_settler
calldata_register: swap_quote
cache_as: swap
```

This sets registers: `swap_function`, `swap_contract`, `swap_value`, `swap_param_0` through `swap_param_4`

### 8. Execute swap
**Only call this AFTER decode_calldata has set the registers above. Do NOT pass a `params` array - the preset reads from registers.**
```tool:web3_function_call
preset: swap_execute
network: base
```

### 9. Verify queued transaction
After queueing, confirm the transaction details:
```tool:list_queued_web3_tx
status: pending
limit: 1
```

### 10. Broadcast when ready
```tool:broadcast_web3_tx
```

---

## Quick Reference: Which Workflow?

| Selling | Workflow | Key Difference |
|---------|----------|----------------|
| ETH | Workflow A | Wrap ETH → WETH first, then swap WETH |
| WETH | Workflow B | No wrapping needed |
| USDC, other tokens | Workflow B | No wrapping needed |

---

## Amount Reference (Wei Values)

For ETH/WETH (18 decimals):
- 0.0001 ETH = `100000000000000`
- 0.001 ETH = `1000000000000000`
- 0.01 ETH = `10000000000000000`
- 0.1 ETH = `100000000000000000`
- 1 ETH = `1000000000000000000`

For USDC (6 decimals):
- 1 USDC = `1000000`
- 10 USDC = `10000000`
- 100 USDC = `100000000`

---

## CRITICAL RULES

### You CANNOT use register_set for these registers:
- `sell_token` - use `token_lookup` with `cache_as: "sell_token"`
- `buy_token` - use `token_lookup` with `cache_as: "buy_token"`

### Always wrap ETH before swapping!
If user says "swap ETH for X", you MUST:
1. Wrap ETH to WETH first (using `weth_deposit` preset)
2. Check WETH allowance for Permit2
3. Approve WETH if allowance is insufficient
4. Then swap WETH for X

### ALWAYS check and set allowance before swapping!
**This prevents "transaction reverted" errors!** The swap contract (via Permit2) needs permission to spend your tokens:
1. Check current allowance using `erc20_allowance` preset
2. If allowance < swap amount, approve using `erc20_approve` preset
3. The spender address is always Permit2: `0x000000000022D473030F116dDEE9F6B43aC78BA3`
4. **WETH is especially prone to zero allowance after wrapping - always check!**

### ALWAYS use decode_calldata before swap_execute!
**The `swap_execute` preset reads from registers, NOT from params you pass.**
1. After getting the swap quote, you MUST call `decode_calldata` with `abi: 0x_settler`
2. This sets the required registers: `swap_contract`, `swap_function`, `swap_value`, `swap_param_0` through `swap_param_4`
3. Only THEN call `web3_function_call` with `preset: swap_execute`
4. **Do NOT try to pass raw calldata or a `params` array to web3_function_call - it will fail!**

---

## Supported Tokens

Use the `token_lookup` tool to check if a token is supported. The tool will return available tokens if the requested one isn't found.

---

## Error Handling

| Error | Fix |
|-------|-----|
| "Cannot set 'sell_token' via register_set" | Use `token_lookup` with `cache_as: "sell_token"` |
| "Cannot set 'buy_token' via register_set" | Use `token_lookup` with `cache_as: "buy_token"` |
| "Preset requires register 'X'" | Run the tool that sets register X first |
| "Insufficient balance" | Check balance before swapping |
| Swap fails with ETH | Make sure you wrapped ETH to WETH first! |
| **TRANSACTION REVERTED (after wrapping WETH)** | **Check allowance! Freshly wrapped WETH has zero allowance. You MUST approve Permit2 (`0x000000000022D473030F116dDEE9F6B43aC78BA3`) before swapping!** |
| **Transaction reverted / Insufficient allowance** | **The DEX cannot spend your tokens. Check `erc20_allowance` and run `erc20_approve` for the Permit2 contract if needed.** |
| **402 Payment Required / Settlement error** | **The tool automatically retries 3 times with a 5 second delay. If it still fails after retries, STOP and inform the user.** |
| **Swap quote fails (any error)** | **The tool automatically retries 3 times. If it fails after all retries, STOP the swap workflow immediately. Tell the user: "Unable to get a swap quote for this trade. The swap cannot be completed."** |
