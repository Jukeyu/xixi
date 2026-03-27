# xixi | Desktop Pet Assistant

xixi is an open-source desktop pet assistant that turns chat commands into real local computer actions.
Our long-term goal is an assistive, hands-free desktop helper for older adults and users with limited mobility.

This is not a mock UI project. We only expose features that are wired to real execution paths.

## Vision

Build a companion that can:

- chat naturally
- understand what is happening on the main screen
- operate the local computer safely through reusable skills
- later accept voice and remote chat commands (WeChat / Feishu / enterprise chat)

We want xixi to become a practical accessibility layer for everyday computer use.

## Current status (as of 2026-03-28)

This repository contains a runnable Windows desktop prototype with real execution paths:

- Tauri + React + TypeScript desktop shell
- Honest command planner (no fake execution), risk labels, action logs
- Tray mode + pet window mode
- Command mode + model chat mode (OpenAI-compatible `/chat/completions`)
- Local skill system (JSON + Python/PowerShell scripts)
- Screen observation stack:
  - `screen_watch_ocr.py`
  - `desktop_snapshot.py`
  - `screen_intent_watch.py`
  - `screen_behavior_watch.py`
  - `latest desktop snapshot`
  - `latest screen watch`
  - `latest screen summary`
  - `run screen suggestion` (auto-selects one safe next action from latest summary)
- Browser/page automation starter:
  - `page agent inspect <url>`
  - `page agent click <url> <text>`
  - `latest page agent`
- Remote bridge v1:
  - local queue polling in desktop app
  - Feishu callback gateway script

## What makes xixi different

- Real command execution with explicit risk levels
- Screen observation stack for intent and behavior signals
- Local skill framework you can extend with your own scripts
- Safety-first boundaries (confirmation gates, allowed paths, audit logs)
- Built as a desktop pet experience, not just a command console

## Why this project

Most desktop software assumes users can navigate complex UI quickly.
xixi aims to make computer usage accessible through natural commands, then progressively through voice.

Target direction:

- Chat-to-Action
- Voice-to-Action
- Skill marketplace
- Assistive computing workflows

## Quick start

```bash
cd apps/desktop
npm install
npm run tauri:dev
```

## Build

```bash
cd apps/desktop
npm run tauri:build
```

Windows build outputs:

- `apps/desktop/src-tauri/target/release/app.exe`
- `apps/desktop/src-tauri/target/release/bundle/msi/...`
- `apps/desktop/src-tauri/target/release/bundle/nsis/...`

## Smoke test

```bash
cd apps/desktop
npm run test:smoke
```

## Useful commands

```text
open site github.com
search web tauri tray icon
open app vscode
open folder downloads

move mouse 960,540
click 960,540
double click 960,540
right click 960,540
drag mouse 760,420 to 1120,640
type hello from xixi
hotkey ctrl,s

watch screen stock
desktop snapshot
latest desktop snapshot
latest desktop cognition
desktop cognition report
screen intent coding
watch screen behavior workflow
latest screen watch
latest screen intent
latest screen behavior
latest screen summary
run screen suggestion

page agent inspect example.com
page agent click example.com More information
latest page agent
```

## Local skills

- Skill folder: `%LOCALAPPDATA%\xixi\skills`
- Script folder: `%LOCALAPPDATA%\xixi\skills\scripts`
- Script run logs: `%LOCALAPPDATA%\xixi\skills\runs`
- Run format: `run skill <skill_id> [input]`

See:

- `docs/skills/local-skills.md`
- `docs/skills/github-research-notes-2026-03-27.md`

### Write your own skill (example)

1) Create a skill JSON:

`%LOCALAPPDATA%\xixi\skills\open_calculator.json`

```json
{
  "id": "open_calculator",
  "name": "Open Calculator",
  "description": "Launch Windows calculator.",
  "kind": "open_app",
  "target_template": "calculator",
  "risk_level": "low-risk",
  "aliases": ["calculator", "open calc"]
}
```

2) Restart xixi (or reload skills).
3) Run:

```text
run skill open_calculator
```

## Remote chat bridge (v1)

Start gateway:

```bash
python scripts/xixi_chat_bridge.py --host 0.0.0.0 --port 17770
```

Manual ingest test:

```bash
curl -X POST http://127.0.0.1:17770/ingest \
  -H "Content-Type: application/json" \
  -d "{\"source\":\"manual\",\"text\":\"/xixi open app calculator\"}"
```

## Python dependencies

For OCR and screen observation:

```bash
pip install mss pillow pytesseract
```

For desktop input automation:

```bash
pip install pyautogui
```

For page automation:

```bash
pip install playwright
python -m playwright install chromium
```

## Safety boundaries (current)

- Only wired commands execute; unsupported requests are explicitly blocked
- `run_script` is restricted to local skills script folder and `.py` / `.ps1`
- High-risk actions require confirmation in UI
- Every action writes structured logs for audit/replay
- `run screen suggestion` only auto-runs low-risk action kinds

## Roadmap focus

- Safer autonomous desktop task chains for non-technical users
- Accessibility-first UX for seniors and low-vision users
- Better multilingual command support
- Voice input/output pipeline
- Community skill packaging and contribution workflow

## How to contribute right now

High-impact contribution areas:

- Screen understanding and intent inference
- Reliable local automation skills (Windows-first)
- Accessibility UX (contrast, readability, keyboard-only flow)
- Safety guardrails and recovery logic
- Integration bridges (Feishu / WeChat / enterprise chat)

## Contributing

Contributions are welcome, especially in:

- Assistive UX and accessibility
- Local automation skills
- Safer action execution and recovery
- Screen understanding and intent inference
- Voice interaction pipeline

Please read `CONTRIBUTING.md` and open an issue or PR.
