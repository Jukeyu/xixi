# Local Skills Guide

`xixi` supports user-defined local skills through JSON files.

## Skill Folder

- Windows: `%LOCALAPPDATA%\xixi\skills`
- You can quickly open it from the app using `open skills folder`.

## Run a Skill

```text
run skill <skill_id> [input]
```

Examples:

```text
run skill open_github
run skill open_tradingview
run skill search_stock_news tsla
```

## JSON Schema (Current)

```json
{
  "id": "search_stock_news",
  "name": "Search Stock News",
  "description": "Search stock news by keyword input.",
  "kind": "search_web",
  "target_template": "{{input}} stock news",
  "label_template": "Stock news: {{input}}",
  "risk_level": "low-risk",
  "aliases": ["stocknews", "股票新闻"]
}
```

## Supported `kind`

- `open_url`
- `search_web`
- `open_folder`
- `open_app`

## Template Rules

- `{{input}}` is optional.
- If a template includes `{{input}}`, the skill must be run with input.
- `search_web` will open browser search if target is not already a full URL.

## Safety Boundary

- Skills only map to existing whitelisted local action kinds.
- Unknown `kind` values are rejected.
- Execution still goes through xixi logging and recovery flow.
