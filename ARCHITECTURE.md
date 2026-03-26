# xixi Architecture

This document describes the current architecture of the runnable desktop build.

## Stack

- Desktop shell: Tauri (Rust)
- UI: React + TypeScript + Vite
- Packaging: MSI + NSIS via Tauri build pipeline

## High-Level Flow

1. User enters a chat command in the React UI.
2. UI calls `plan_user_request` in Rust through Tauri `invoke`.
3. Planner returns:
   - execution eligibility
   - structured steps
   - optional concrete local action payload
4. If execution is allowed and auto-run is enabled:
   - UI calls `execute_local_action`
   - Rust executor runs the native command
5. UI updates queue state and appends result messages.

## Frontend Responsibilities

`apps/desktop/src/App.tsx` handles:

- chat transcript state
- action queue rendering
- theme and settings persistence
- context menu and settings panel
- weather data fetch and display
- desktop window controls (minimize, maximize, close)

`apps/desktop/src/App.css` and `src/index.css` provide:

- high-contrast UI styling
- dark/light theme tokens
- responsive layout behavior

## Backend Responsibilities

`apps/desktop/src-tauri/src/lib.rs` handles:

- command planning (`plan_user_request`)
- execution dispatch (`execute_local_action`)
- executable adapters:
  - folder open
  - URL open
  - app launch (Chrome, Edge, Notepad, Explorer)
- explicit unsupported-command handling

## Safety Model (Current)

- Only whitelisted actions are executable.
- Unknown commands return a not-implemented plan.
- No destructive file operations are exposed in current executor.
- Unsupported requests are visibly reported, not silently ignored.

## Test Model

- Type check: `npm run check`
- Frontend build: `npm run build`
- Rust unit tests: `npm run test:rust`
- Combined smoke gate: `npm run test:smoke`
- CI gate: `.github/workflows/ci.yml`

## Extension Direction

Planned architecture extensions:

- richer command parser (entities + parameters)
- safer multi-step task confirmation for medium/high-risk actions
- installable skill registry
- specialist agent routing layer
- execution telemetry and replayable action logs
