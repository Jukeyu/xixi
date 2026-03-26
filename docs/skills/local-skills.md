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
run skill screen_watch_stub stock
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
- `run_script`

## Template Rules

- `{{input}}` is optional.
- If a template includes `{{input}}`, the skill must be run with input.
- `search_web` will open browser search if target is not already a full URL.
- `run_script` stores script + input payload and executes local `.py` or `.ps1`.

## Script Skills (`run_script`)

- Script folder: `%LOCALAPPDATA%\xixi\skills\scripts`
- Current allowed extensions: `.py`, `.ps1`
- Sample script created automatically: `sample_screen_watch.py`

Example:

```json
{
  "id": "screen_watch_stub",
  "name": "Screen Watch Stub",
  "description": "Run a local python stub script for screen-watch workflow.",
  "kind": "run_script",
  "target_template": "sample_screen_watch.py",
  "label_template": "Screen Watch Stub",
  "risk_level": "medium-risk",
  "aliases": ["watchscreen", "盯屏"]
}
```

When you run:

```text
run skill screen_watch_stub stock
```

xixi starts the script and passes `stock` as the first argument.

## Reusing Skills from Internet

You can reuse ideas from online projects, but do not copy blindly.

Checklist before using external skill code:

1. Check license compatibility (MIT/Apache preferred).
2. Read the script and remove destructive operations.
3. Keep scripts inside local skills folder only.
4. Test with non-sensitive data first.
5. Add explicit risk level and recovery notes.

## Safety Boundary

- Skills only map to existing whitelisted local action kinds.
- Unknown `kind` values are rejected.
- Execution still goes through xixi logging and recovery flow.
- `run_script` is restricted to local skills script folder and allowed extensions.
