---
name: dexscreener
description: "Get DEX token prices, pair info, liquidity data, and trending tokens from DexScreener. Supports all major chains."
version: 1.0.0
author: starkbot
homepage: https://docs.dexscreener.com/api/reference
metadata: {"clawdbot":{"emoji":"📈"}}
requires_tools: [dexscreener]
tags: [crypto, dex, price, token, liquidity, trading, defi, market-data]
arguments:
  query:
    description: "Search query (token name, symbol, or address)"
    required: false
  chain:
    description: "Chain (ethereum, base, solana, bsc, polygon, arbitrum, etc.)"
    required: false
  address:
    description: "Token or pair contract address"
    required: false
---

# DexScreener Market Data

Use the `dexscreener` tool to get real-time DEX trading data across all major chains.

## Tool Actions

### 1. Search for Tokens

Find tokens by name, symbol, or address:

```json
{"tool": "dexscreener", "action": "search", "query": "PEPE"}
```

```json
{"tool": "dexscreener", "action": "search", "query": "0x6982508145454ce325ddbe47a25d4ec3d2311933"}
```

### 2. Get Token by Address

Get all trading pairs for a specific token:

```json
{"tool": "dexscreener", "action": "token", "chain": "base", "address": "0x532f27101965dd16442e59d40670faf5ebb142e4"}
```

### 3. Get Pair/Pool Info

Get details for a specific liquidity pool:

```json
{"tool": "dexscreener", "action": "pair", "chain": "ethereum", "address": "0x..."}
```

### 4. Get Trending Tokens

See tokens with the most boosts (paid promotions - often new launches):

```json
{"tool": "dexscreener", "action": "trending"}
```

---

## Supported Chains

| Chain | ID |
|-------|-----|
| Ethereum | `ethereum` |
| Base | `base` |
| Solana | `solana` |
| BSC | `bsc` |
| Polygon | `polygon` |
| Arbitrum | `arbitrum` |
| Optimism | `optimism` |
| Avalanche | `avalanche` |

---

## Understanding the Output

The tool returns formatted data including:

- **Price** - Current USD price with 24h change %
- **MCap** - Market capitalization
- **Liquidity** - Total liquidity in USD (important for slippage)
- **24h Vol** - Trading volume
- **24h Txns** - Buy/sell transaction counts
- **Token address** - Contract address
- **DexScreener URL** - Link to chart

---

## Common Workflows

### Check Token Price

User asks: "What's the price of PEPE?"

```json
{"tool": "dexscreener", "action": "search", "query": "PEPE"}
```

Report the price, 24h change, and liquidity from the top result.

### Research a Token Address

User provides a contract address:

```json
{"tool": "dexscreener", "action": "token", "chain": "base", "address": "0x..."}
```

Check:
- Is there liquidity? (>$50K is safer)
- Trading activity (buys vs sells)
- Price volatility

### Find New/Trending Tokens

```json
{"tool": "dexscreener", "action": "trending"}
```

Note: Trending = paid boosts. High risk, often new launches.

---

## Tips

1. **Multiple pairs** - Tokens often have multiple pools; the tool shows the top ones sorted by liquidity
2. **Low liquidity warning** - If liquidity is under $10K, warn user about high slippage
3. **Chain matters** - Same token name can exist on different chains; verify the chain
4. **Search is fuzzy** - Works with partial matches and addresses
