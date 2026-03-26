# xixi / 晰晰

`xixi` is an open-source desktop AI assistant for non-coders.

This project is building a real desktop companion that can chat, explain actions clearly, and run safe local automation on Windows.
It is not a toy chat mockup: every supported command in the UI is backed by executable code.

## Why This Project Matters

Most AI apps can talk but cannot operate a real personal computer.
Most automation tools can operate a computer but are hard for beginners.

`xixi` combines both:

- a desktop-first AI chat workspace
- transparent action planning
- safe local command execution
- settings that normal users can understand
- a contributor-friendly architecture for skills and specialist agents

## Current Real Capabilities (March 26, 2026)

The current build can:

- run as a real Windows desktop app (Tauri)
- minimize / maximize / close via real desktop window APIs
- switch light/dark theme and persist settings
- show live weather data from Open-Meteo
- parse and execute supported desktop commands
- reject unsupported commands honestly (no fake success)

Supported command examples today:

- `Open QMDownload`
- `Open xixi folder`
- `Open GitHub`
- `Open weather`
- `Open Chrome`
- `Open Edge`
- `Open Notepad`
- `Open Explorer`

Chinese phrase support is also wired for common variants like:

- `打开 github`
- `打开记事本`
- `打开资源管理器`

## Product Direction

Goal: build a desktop AI pet that feels alive and useful, not gimmicky.

Core direction:

- pet presence + chat workspace
- natural language to desktop actions
- clear safety behavior (small tasks direct, risky tasks confirm)
- plug-in points for skills and specialist agents
- beginner-friendly UX and explanations

## Quick Start

```bash
cd apps/desktop
npm install
npm run tauri:dev
```

Build packages:

```bash
cd apps/desktop
npm run tauri:build
```

## Publish to GitHub

Once `gh auth login` is completed on this machine, publish with:

```bash
powershell -ExecutionPolicy Bypass -File .\scripts\publish-github.ps1 -RepoName xixi
```

This script will:

- create the GitHub repository (if `origin` does not exist)
- set `origin`
- push `main`

## Test Workflow

Local smoke test:

```bash
cd apps/desktop
npm run test:smoke
```

This runs:

- TypeScript checks
- frontend build
- Rust unit tests

CI is configured in:

- `.github/workflows/ci.yml`

## Repo Structure

- `apps/desktop`: Tauri + React desktop app
- `docs/design`: design records
- `docs/plans`: implementation plans
- `.github/workflows`: CI pipeline
- `ARCHITECTURE.md`: runtime architecture overview
- `CONTRIBUTING.md`: contribution workflow
- `ROADMAP.md`: staged project milestones

## Contributing

This project is intentionally open for collaboration.
If you are strong in any of these areas, your help is high impact:

- desktop automation
- LLM command planning
- Tauri and Rust systems work
- React UX for non-technical users
- skill/plugin architecture
- QA and safety validation

Good first contribution directions:

1. Add one new real desktop action end-to-end (planner + executor + UI trace).
2. Expand multilingual intent parsing while keeping strict execution honesty.
3. Improve accessibility and readability for low-vision users.
4. Build skill registry and external skill packaging flow.
5. Add deterministic test cases for edge command phrasing.

## Open Collaboration Call

If you want to help build a practical AI desktop assistant that beginners can actually use, join this project.

This is not about flashy demos.
This is about shipping a reliable, understandable, open desktop AI system together.
