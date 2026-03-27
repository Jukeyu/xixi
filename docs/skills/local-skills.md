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
run skill open_firefox
run skill open_vscode
run skill open_terminal
run skill open_music_player
run skill open_tradingview
run skill search_stock_news tsla
run skill screen_watch_ocr keyword=stock duration=20
run skill screen_intent_watch goal=trading duration=16 samples=6
run skill desktop_action_safe click
run skill desktop_skill_ops rightclick
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
- Script run logs: `%LOCALAPPDATA%\xixi\skills\runs`
- Current allowed extensions: `.py`, `.ps1`
- Built-in templates created automatically:
  - `screen_watch_ocr.py`
  - `screen_intent_watch.py`
  - `safe_desktop_action.py`

Example (screen watch):

```json
{
  "id": "screen_watch_ocr",
  "name": "Screen Watch OCR",
  "description": "Watch screen OCR text and detect keyword hits.",
  "kind": "run_script",
  "target_template": "screen_watch_ocr.py",
  "label_template": "Screen Watch OCR",
  "risk_level": "medium-risk",
  "aliases": ["watchocr", "盯屏识别"]
}
```

When you run:

```text
run skill screen_watch_ocr keyword=stock duration=20 interval=1 max_hits=2
```

xixi starts the script and passes the full input string as script arg #1.

`screen_watch_ocr.py` option keys:

- `keyword`: keyword to detect in OCR text
- `duration`: total seconds to watch
- `interval`: seconds between scans
- `max_hits`: stop after N hits
- `region`: optional `left,top,width,height`

`screen_intent_watch.py` option keys:

- `goal`: optional goal hint (for intent scoring)
- `duration`: total observation seconds
- `interval`: seconds between each sample
- `samples`: max sample count
- `max_chars`: OCR text cap per sample
- `ocr`: `1|0` to enable or disable OCR capture
- `region`: optional `left,top,width,height`

Example:

```text
run skill screen_intent_watch goal=coding duration=18 interval=1.2 samples=8
```

Example (desktop action):

```text
run skill desktop_action_safe click
run skill desktop_action_safe move:960,540
run skill desktop_action_safe type:hello from xixi
run skill desktop_action_safe hotkey:ctrl,s
run skill desktop_skill_ops rightclick
run skill desktop_skill_ops scroll:-400
run skill desktop_skill_ops wait:1.2
```

`desktop_action_safe` blocks a small set of dangerous combinations by default and logs every run.
Because it is marked `high-risk`, the desktop UI requires manual confirmation before execution.

You can also use direct natural command phrases in chat (without `run skill`), for example:

```text
open music player
open app firefox
open app vscode
open terminal
type hello from xixi
press key enter
hotkey ctrl,s
watch screen stock
screen intent coding
watch intent trading
move mouse 960,540
right click
scroll down 400
```

## Python Dependencies

For `screen_watch_ocr.py` and `screen_intent_watch.py`:

```text
pip install mss pillow pytesseract
```

For `desktop_action_safe.py`:

```text
pip install pyautogui
```

If a dependency is missing, the script exits and writes guidance to run logs.

## Reusing Skills from Internet

You can reuse ideas from online projects, but do not copy blindly.

Current evaluated sources are tracked in:

- `docs/skills/github-research-notes-2026-03-27.md`

Checklist before using external skill code:

1. Check license compatibility (MIT/Apache preferred).
2. Read the script and remove destructive operations.
3. Keep scripts inside local skills folder only.
4. Test with non-sensitive data first.
5. Add explicit risk level and recovery notes.
6. Keep a changelog in script header to track what you adapted.

## Safety Boundary

- Skills only map to existing whitelisted local action kinds.
- Unknown `kind` values are rejected.
- Execution still goes through xixi logging and recovery flow.
- `run_script` is restricted to local skills script folder and allowed extensions.
- `run_script` output is redirected to `%LOCALAPPDATA%\xixi\skills\runs` for auditing.
- UI permission profiles can block skill execution before action starts.
