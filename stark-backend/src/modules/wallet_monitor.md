---
name: wallet_monitor
description: "Monitor ETH wallets for on-chain activity, detect whale trades, and track transaction history on Ethereum Mainnet and Base"
version: 1.1.0
author: starkbot
tags: [crypto, defi, monitoring, wallets, whale, alerts]
requires_tools: [wallet_watchlist, wallet_activity, wallet_monitor_control, dexscreener, token_lookup]
---

# Wallet Monitor Skill

You are helping the user manage their wallet monitoring setup. This skill tracks on-chain activity for watched wallets using Alchemy Enhanced APIs, detecting transfers, swaps, and large trades on Ethereum Mainnet and Base.

The wallet monitor runs as a separate microservice. All tool calls communicate with it via RPC.

## Available Tools

1. **wallet_watchlist** — Manage the list of watched wallets
   - `add`: Add a new wallet to monitor (requires address, optional label/chain/threshold)
   - `remove`: Remove a wallet by ID
   - `list`: Show all watched wallets
   - `update`: Modify wallet settings (label, threshold, enable/disable)

2. **wallet_activity** — Query logged on-chain activity
   - `recent`: Show recent transactions across all watched wallets
   - `large_trades`: Show only large trades (above threshold)
   - `search`: Filter by address, chain, activity type
   - `stats`: Overview statistics

3. **wallet_monitor_control** — Control the background worker
   - `status`: Check if the monitor service is running, wallet counts, uptime
   - `trigger`: Verify worker is active (polls every 60s automatically)

## Workflow

1. First check status: `wallet_monitor_control(action="status")`
2. Add wallets: `wallet_watchlist(action="add", address="0x...", label="Whale Alpha", chain="mainnet", threshold_usd=50000)`
3. The background worker automatically polls every 60 seconds
4. Query activity: `wallet_activity(action="recent")` or `wallet_activity(action="large_trades")`

## Important Notes

- The wallet monitor runs as a standalone service (wallet-monitor-service)
- Dashboard available at http://127.0.0.1:9100/
- Supported chains: "mainnet" (Ethereum) and "base" (Base)
- Each wallet has its own large_trade_threshold_usd (default $1,000)
- Swap detection: transactions with both outgoing and incoming ERC-20 transfers are classified as swaps
- USD values are estimated using DexScreener price data (cached 60s)
- The worker uses block-number cursors for gap-free incremental polling
