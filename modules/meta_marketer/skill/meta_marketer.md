---
name: meta_marketer
description: "Manage Meta (Facebook/Instagram) ad campaigns — create campaigns, monitor performance, audit spend, and optimize ROAS/CPA"
version: 1.0.0
author: starkbot
requires_tools: [meta_ads, meta_insights, kv_store]
tags: [marketing, meta, ads, facebook, instagram]
---

# Meta Marketer — Tool Reference

The `meta_marketer` module provides two tools for managing Meta advertising: `meta_ads` for campaign CRUD and `meta_insights` for performance analytics.

## meta_ads — Campaign Management

Manage campaigns, ad sets, ads, and creatives.

### List Campaigns

```
meta_ads(action="list_campaigns", limit=25)
```

### Create a Campaign

All campaigns are created in **PAUSED** state for safety.

```
meta_ads(action="create_campaign", config="{\"name\": \"Summer Sale 2026\", \"objective\": \"OUTCOME_SALES\"}")
```

**Supported objectives:** `OUTCOME_AWARENESS`, `OUTCOME_ENGAGEMENT`, `OUTCOME_TRAFFIC`, `OUTCOME_LEADS`, `OUTCOME_APP_PROMOTION`, `OUTCOME_SALES`

### Create an Ad Set

Budget values are in the account's currency (e.g. cents for USD accounts — $50/day = 5000).

```
meta_ads(action="create_adset", campaign_id="CAMPAIGN_ID", config="{
  \"name\": \"US Women 25-44\",
  \"daily_budget\": 5000,
  \"optimization_goal\": \"OFFSITE_CONVERSIONS\",
  \"billing_event\": \"IMPRESSIONS\",
  \"targeting\": {
    \"geo_locations\": {\"countries\": [\"US\"]},
    \"age_min\": 25,
    \"age_max\": 44,
    \"genders\": [2]
  }
}")
```

### Create an Ad Creative

```
meta_ads(action="create_creative", config="{
  \"name\": \"Summer Sale Image Ad\",
  \"object_story_spec\": {
    \"page_id\": \"PAGE_ID\",
    \"link_data\": {
      \"image_hash\": \"IMAGE_HASH\",
      \"link\": \"https://example.com/sale\",
      \"message\": \"50% off everything this weekend!\",
      \"name\": \"Summer Sale\",
      \"description\": \"Shop now before it's gone\",
      \"call_to_action\": {\"type\": \"SHOP_NOW\"}
    }
  }
}")
```

### Create an Ad

```
meta_ads(action="create_ad", adset_id="ADSET_ID", config="{
  \"name\": \"Summer Sale - Image Variant A\",
  \"creative\": {\"creative_id\": \"CREATIVE_ID\"}
}")
```

### Update & Pause

```
meta_ads(action="update_campaign", campaign_id="ID", config="{\"daily_budget\": 7500}")
meta_ads(action="pause_campaign", campaign_id="ID")
meta_ads(action="update_adset", adset_id="ID", config="{\"daily_budget\": 3000}")
meta_ads(action="update_ad", ad_id="ID", config="{\"status\": \"ACTIVE\"}")
```

## meta_insights — Performance Analytics

Pull spend, impressions, clicks, conversions, CPA, ROAS with optional breakdowns.

### Account-Level Insights

```
meta_insights(action="account_insights", date_preset="last_7d")
```

### Campaign-Level Insights

```
meta_insights(action="campaign_insights", date_preset="last_7d")
meta_insights(action="campaign_insights", campaign_id="ID", date_preset="last_30d")
```

### With Breakdowns

```
meta_insights(action="adset_insights", campaign_id="ID", date_preset="last_7d", breakdowns="age,gender")
```

**Available breakdowns:** `age`, `gender`, `placement`, `device`, `country`

### Custom Date Range

```
meta_insights(action="campaign_insights", time_range="{\"since\": \"2026-01-01\", \"until\": \"2026-01-31\"}")
```

### Full Account Audit

Pulls all active campaigns and flags issues against your targets.

```
meta_insights(action="audit", target_cpa=45.00, target_roas=4.0, date_preset="last_7d")
```

Returns:
- **summary**: total spend, conversions, avg CPA, issue count
- **campaigns**: per-campaign metrics (spend, impressions, clicks, CTR, conversions, CPA, ROAS)
- **issues**: flagged problems ranked by severity with recommended actions

Issue types detected:
- `CPA_OVER_TARGET` — CPA exceeds your target
- `ROAS_BELOW_TARGET` — ROAS below your target
- `LOW_CTR` — CTR <0.5% with significant spend (creative fatigue)
- `ZERO_CONVERSIONS` — money spent with no conversions

### Date Presets

`today`, `yesterday`, `last_3d`, `last_7d`, `last_14d`, `last_30d`, `last_90d`, `this_month`, `last_month`

## State Tracking with kv_store

The meta_marketer agent uses kv_store to track state between sessions:

| Key Pattern | Purpose |
|-------------|---------|
| `META_LAST_AUDIT_TS` | Timestamp of last full audit |
| `META_DAILY_SPEND_{date}` | Daily spend tracking |
| `META_CAMPAIGN_{id}_CPA_TREND` | CPA direction per campaign |
| `META_ALERT_{type}_{id}` | Alert dedup to avoid repeat warnings |
| `META_CHANGE_LOG_{ts}` | Log of all changes made |

## Safety Model

- **Read operations** (list, get, insights, audit) execute freely
- **Write operations** (create, update, pause) should be confirmed with the user before execution
- All new campaigns and ad sets are created **PAUSED** — review before activating
- Budget increases are capped at 30% per change to protect Meta's learning phase
