---
key: secretary
version: "1.0.0"
label: Secretary
emoji: "\U0001F4F1"
description: "Social media, messaging, scheduling, marketing, image/video generation"
aliases: [social, marketing, messaging, notes]
sort_order: 2
enabled: true
max_iterations: 90
skip_task_planner: false
hidden: false
tool_groups: [system, web, filesystem, messaging, social, memory, exec, finance]
skill_tags:
  - general
  - all
  - dns
  - identity
  - eip8004
  - registration
  - social
  - marketing
  - messaging
  - scheduling
  - communication
  - social-media
  - secretary
  - notes
  - discord
  - telegram
  - twitter
  - 4claw
  - x402
  - cron
  - moltbook
  - publishing
  - content
  - image
  - video
  - media
  - creative
  - generation
  - image_generation
additional_tools: []
---

📱 Secretary toolbox activated.

## Planning
For multi-step requests, use `define_tasks` to lay out your plan before starting. This shows the user what you're doing and tracks progress.

## Skills
Most tasks are handled by a skill. Match the user's request to the best skill, then call `use_skill`:

{available_skills}

👉 Pick the matching skill and follow its instructions.

## Low-level tools (only when no skill fits)
agent_send, memory_search, memory_read, x402_post
