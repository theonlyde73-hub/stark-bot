---
name: transfer_erc20
description: "Transfer (Send) ERC20 tokens on Base/Ethereum using the burner wallet"
version: 1.0.0
author: starkbot
homepage: https://basescan.org
metadata: {"requires_auth": false, "clawdbot":{"emoji":"🪙"}}
tags: [crypto, transfer, send, erc20, tokens, base, wallet]
requires_tools: [web, x, register_set, token_lookup, to_raw_amount]
---

# ERC20 Token Transfer/Send Skill

Transfer or Send ERC20 tokens from the burner wallet to any address.

> **IMPORTANT: This skill uses the REGISTER PATTERN to prevent hallucination of transaction data.**
>
> - Use `token_lookup` to get token address and decimals
> - Use `to_raw_amount` to convert human amounts to raw units
> - The `transfer_amount` register is validated by `web3_function_call`

## Tools Used

| Tool | Purpose |
|------|---------|
| `token_lookup` | Get token address and decimals |
| `to_raw_amount` | Convert human amount to raw units safely |
| `web3_function_call` | Execute ERC20 transfers and check balances |
| `register_set` | Set token address for balance checks |

**Note:** `wallet_address` is an intrinsic register - always available automatically.

---

## Required Tool Flow

**ALWAYS follow this sequence for ERC20 transfers:**

1. `token_lookup` → Get token address and decimals
2. `to_raw_amount` → Convert human amount to raw units
3. `web3_function_call` → Execute the transfer

---

## Step 1: Look up the token

```tool:token_lookup
symbol: "STARKBOT"
network: base
cache_as: token_address
```

This sets registers:
- `token_address` → contract address
- `token_address_decimals` → decimals (e.g., 18)

---

## Step 2: Convert amount to raw units

```tool:to_raw_amount
amount: "1"
cache_as: "transfer_amount"
```

This reads `token_address_decimals` automatically and sets:
- `transfer_amount` → "1000000000000000000" (for 18 decimals)

---

## Step 3: Execute the transfer

```tool:web3_function_call
abi: erc20
contract: "<TOKEN_ADDRESS from step 1>"
function: transfer
params: ["<RECIPIENT_ADDRESS>", "<RAW_AMOUNT from step 2>"]
network: base
```

---

## Complete Example: Send 10 USDC

```tool:token_lookup
symbol: "USDC"
network: base
cache_as: token_address
```

```tool:to_raw_amount
amount: "10"
cache_as: "transfer_amount"
```

```tool:web3_function_call
abi: erc20
contract: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
function: transfer
params: ["0x1234567890abcdef1234567890abcdef12345678", "10000000"]
network: base
```

> **Note:** The `transfer_amount` register is validated by `web3_function_call` to prevent hallucinated amounts.

---

## Verify and Broadcast

After queueing, verify the transaction:
```tool:list_queued_web3_tx
status: pending
limit: 1
```

Broadcast when ready:
```tool:broadcast_web3_tx
```

---

## Check ERC20 Token Balance

First set the token address, then use the erc20_balance preset:

```tool:register_set
key: token_address
value: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
```

```tool:web3_function_call
preset: erc20_balance
network: base
call_only: true
```

---

## Common Token Addresses (Base)

Use `token_lookup` to get addresses automatically, or use these directly:

| Token | Address | Decimals |
|-------|---------|----------|
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | 6 |
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| BNKR | `0x22aF33FE49fD1Fa80c7149773dDe5890D3c76F3b` | 18 |
| cbBTC | `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf` | 8 |
| DAI | `0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb` | 18 |
| USDbC | `0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA` | 6 |

---

## Amount Conversion Reference

| Token | Decimals | Human Amount | Raw Value |
|-------|----------|--------------|-----------|
| USDC | 6 | 1 | `1000000` |
| USDC | 6 | 10 | `10000000` |
| USDC | 6 | 100 | `100000000` |
| BNKR | 18 | 1 | `1000000000000000000` |
| BNKR | 18 | 100 | `100000000000000000000` |
| cbBTC | 8 | 0.001 | `100000` |
| cbBTC | 8 | 0.01 | `1000000` |

---

## Pre-Transfer Checklist

Before executing a transfer:

1. **Verify recipient address** - Double-check the address is correct
2. **Check balance** - Use `web3_function_call` (erc20_balance preset) for tokens
3. **Confirm amount** - Ensure decimals are correct for the token (use `to_raw_amount`!)
4. **Network** - Confirm you're on the right network (base vs mainnet)
5. **ETH for gas** - You need ETH to pay for gas, even when sending ERC20s

---

## Error Handling

| Error | Cause | Solution |
|-------|-------|----------|
| "Insufficient funds" | Not enough ETH for gas | Add ETH to wallet |
| "Transfer amount exceeds balance" | Not enough tokens | Check token balance |
| "Gas estimation failed" | Invalid recipient or params | Verify addresses |
| "Transaction reverted" | Contract rejection | Check amounts |
| "Register not found" | Missing register | Use token_lookup/to_raw_amount first |

---

## Security Notes

1. **Register pattern prevents hallucination** - tx data flows through registers
2. **to_raw_amount validates amounts** - prevents incorrect decimal conversions
3. **Always double-check addresses** - Transactions cannot be reversed
4. **Start with small test amounts** - Verify the flow works first
5. **Verify token contracts** - Use official addresses from block explorer
6. **Gas costs** - ETH needed for gas even when sending ERC20s
