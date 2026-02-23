---
key: finance
version: "1.0.0"
label: Finance
emoji: "\U0001F4B0"
description: "Crypto swaps, transfers, DeFi operations, token lookups"
aliases: [defi, crypto, swap, transfer]
sort_order: 0
enabled: true
max_iterations: 90
skip_task_planner: false
hidden: false
tool_groups: [system, web, filesystem, finance]
skill_tags:
  - general
  - all
  - identity
  - eip8004
  - registration
  - crypto
  - defi
  - transfer
  - swap
  - finance
  - wallet
  - token
  - bridge
  - lending
  - yield
  - dex
  - payments
  - x402
  - transaction
  - polymarket
  - prediction-markets
  - trading
  - price
  - discord
  - tipping
additional_tools: []
---

💰 Finance toolbox activated.

## Planning
For multi-step requests, use `define_tasks` to lay out your plan before starting. This shows the user what you're doing and tracks progress.

## Skills
Most tasks are handled by a skill. Match the user's request to the best skill, then call `use_skill`:

{available_skills}

👉 Pick the matching skill and follow its instructions. Skills define the full workflow including which tools to call and in what order.

## Low-level tools (only when no skill fits)
select_web3_network, web3_tx, web3_function_call, token_lookup, x402_rpc, set_address, ask_user
