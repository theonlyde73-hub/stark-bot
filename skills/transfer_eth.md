---
name: transfer_eth
description: "Transfer (Send) native ETH on Base/Ethereum using the burner wallet"
version: 1.0.0
author: starkbot
homepage: https://basescan.org
metadata: {"requires_auth": false, "clawdbot":{"emoji":"💸"}}
tags: [crypto, transfer, send, eth, base, wallet]
requires_tools: [web, x, register_set]
---

# ETH Transfer/Send Skill

Transfer or Send native ETH from the burner wallet to any address.

> **IMPORTANT: This skill uses the REGISTER PATTERN to prevent hallucination of transaction data.**
>
> - Use `register_set` to store tx data, then `send_eth` reads from the register
> - You NEVER pass raw tx params directly to tools

## Tools Used

| Tool | Purpose |
|------|---------|
| `x402_rpc` | Get gas price and ETH balance (get_balance preset) |
| `register_set` | Build transaction params in a register |
| `send_eth` | Execute native ETH transfers (reads from register) |

**Note:** `wallet_address` is an intrinsic register - always available automatically.

---

## How to Transfer ETH

### Step 1: Build Transfer in Register

Use `register_set` with `json_value` to store the transfer data:

```tool:register_set
key: transfer_tx
json_value:
  to: "<RECIPIENT_ADDRESS>"
  value: "<AMOUNT_IN_WEI>"
```

### Step 2: Queue Transfer

```tool:send_eth
from_register: transfer_tx
network: base
```

Gas is auto-estimated (21000 for simple ETH transfers).

### Step 3: Verify and Broadcast

Verify the queued transaction:
```tool:list_queued_web3_tx
status: pending
limit: 1
```

Broadcast when ready:
```tool:broadcast_web3_tx
```

---

## Complete Example: Send 0.01 ETH

### 1. Build transfer in register

```tool:register_set
key: transfer_tx
json_value:
  to: "0x1234567890abcdef1234567890abcdef12345678"
  value: "10000000000000000"
```

### 2. Queue Transfer

```tool:send_eth
from_register: transfer_tx
network: base
```

### 3. Verify and Broadcast

```tool:list_queued_web3_tx
status: pending
limit: 1
```

```tool:broadcast_web3_tx
```

---

## Check ETH Balance

```tool:x402_rpc
preset: get_balance
network: base
```

The result is hex wei - convert to ETH by dividing by 10^18.

---

## ETH Amount Reference (Wei Values)

| Human Amount | Wei Value |
|--------------|-----------|
| 0.0001 ETH | `100000000000000` |
| 0.001 ETH | `1000000000000000` |
| 0.01 ETH | `10000000000000000` |
| 0.1 ETH | `100000000000000000` |
| 1 ETH | `1000000000000000000` |

---

## Pre-Transfer Checklist

Before executing a transfer:

1. **Verify recipient address** - Double-check the address is correct
2. **Check balance** - Use `x402_rpc` (get_balance) for ETH
3. **Confirm amount** - Ensure wei conversion is correct (18 decimals)
4. **Network** - Confirm you're on the right network (base vs mainnet)

---

## Error Handling

| Error | Cause | Solution |
|-------|-------|----------|
| "Insufficient funds" | Not enough ETH for gas + value | Add ETH to wallet |
| "Gas estimation failed" | Invalid recipient or params | Verify addresses |
| "Transaction reverted" | Should not happen for simple ETH transfer | Check recipient is not a contract that rejects ETH |
| "Register not found" | Missing register | Use register_set first |

---

## Security Notes

1. **Register pattern prevents hallucination** - tx data flows through registers
2. **Always double-check addresses** - Transactions cannot be reversed
3. **Start with small test amounts** - Verify the flow works first
4. **Gas costs** - ETH needed for gas (21000 gas for simple transfer)
