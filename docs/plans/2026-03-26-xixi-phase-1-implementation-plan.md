# xixi Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use `subagent-driven-development` when available for implementation. Steps use checkbox syntax for tracking.

**Goal:** Build the first working version of xixi: a Windows desktop AI pet with a floating orange-cat shell, a maximized GPT-style chat window, natural-language task routing, and real local computer actions for browser, app-launch, and D-drive workflows.

**Architecture:** Use an ability-first desktop architecture. Build a Tauri shell with two windows: a lightweight floating pet window and a maximized chat window. Route all user requests through a central orchestrator that classifies task type, selects persona and model family, then delegates to constrained local executors for browser, app-launch, and D-drive actions.

**Tech Stack:** Tauri, React, TypeScript, Rust commands, local JSON/SQLite storage, browser automation adapter, AutoHotkey/Power Automate adapters, Git/GitHub for project history.

---

## File Structure

Planned project structure:

- `README.md`
  - Public-facing GitHub introduction, contributor pitch, and non-coder framing
- `.gitignore`
  - Ignore build artifacts, local data, and runtime secrets
- `package.json`
  - Frontend scripts and tooling
- `src-tauri/`
  - Native desktop shell, command bridge, window lifecycle, secure local execution
- `src/`
  - Frontend application code
- `src/app/`
  - App shell, routes, top-level layout
- `src/features/chat/`
  - Chat window, input bar, transcript, action results
- `src/features/pet/`
  - Floating pet shell, pet states, self-talk bubble
- `src/features/personas/`
  - Persona switcher and persona definitions
- `src/features/tasks/`
  - Task classification, task objects, action queue, safety handling
- `src/features/executors/`
  - Browser executor, app launcher, file executor, future desktop automation adapters
- `src/features/models/`
  - Model routing configuration and provider abstraction
- `src/features/skills/`
  - Skill and specialist registry
- `src/shared/`
  - Shared types, utilities, storage helpers
- `docs/design/`
  - Phase-one design doc
- `docs/plans/`
  - This implementation plan
- `docs/notes/`
  - Future experiments, integration notes, contributor docs

This structure keeps windows, reasoning, execution, personas, and extensibility separate.

## Chunk 1: Project Foundation and GitHub Positioning

### Task 1: Create the repository skeleton

**Files:**
- Create: `D:\QMDownload\xixi\.gitignore`
- Create: `D:\QMDownload\xixi\README.md`
- Create: `D:\QMDownload\xixi\package.json`
- Create: `D:\QMDownload\xixi\src\`
- Create: `D:\QMDownload\xixi\src-tauri\`

- [ ] **Step 1: Write the README first**

README must include:

- xixi name and concept
- "AI automation desktop companion for non-coders"
- small orange cat identity
- maximized chat window concept
- natural-language computer control
- plugin and skill architecture vision
- invitation for GitHub contributors

- [ ] **Step 2: Write `.gitignore`**

Include ignores for:

- Node modules
- Tauri build outputs
- local env and secrets
- runtime logs
- local automation artifacts
- temp files

- [ ] **Step 3: Initialize frontend package manifest**

Run commands after file creation:

```bash
cd D:\QMDownload\xixi
npm init -y
```

Expected:

- `package.json` exists
- project name is updated to `xixi`

- [ ] **Step 4: Initialize Git repository**

Run:

```bash
cd D:\QMDownload\xixi
git init
git branch -M main
```

Expected:

- repository initialized
- branch is `main`

- [ ] **Step 5: First commit**

Run:

```bash
git add .
git commit -m "chore: initialize xixi project foundation"
```

## Chunk 2: Desktop Shell and Window Model

### Task 2: Scaffold the Tauri desktop app

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Modify: `package.json`

- [ ] **Step 1: Scaffold Tauri**

Run the official Tauri initialization flow appropriate for the chosen frontend.

Expected:

- app runs locally
- basic desktop window opens

- [ ] **Step 2: Define dual-window architecture**

Implement:

- floating pet window
- maximized main chat window
- command bridge between the two windows

- [ ] **Step 3: Add open-chat behavior**

Double-clicking the pet must open or focus the main chat window.

- [ ] **Step 4: Verify**

Run the app locally.

Expected:

- floating window is visible
- double-click focuses the main window

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: scaffold desktop shell and dual-window model"
```

## Chunk 3: GPT-Style Chat Experience

### Task 3: Build the main chat interface

**Files:**
- Create: `src/features/chat/ChatWindow.tsx`
- Create: `src/features/chat/MessageList.tsx`
- Create: `src/features/chat/Composer.tsx`
- Create: `src/features/chat/types.ts`
- Create: `src/app/App.tsx`

- [ ] **Step 1: Render the maximized chat layout**

The first layout must support:

- header area
- persona selector placeholder
- conversation list
- input composer
- status line for task execution

- [ ] **Step 2: Add local chat state**

Support:

- user messages
- assistant messages
- system action messages

- [ ] **Step 3: Add streaming-friendly layout**

Even if live model streaming is not complete yet, the UI should be designed to support partial response rendering later.

- [ ] **Step 4: Verify**

Run the app and manually confirm:

- message list scrolls correctly
- input is stable
- layout resembles a modern AI assistant

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: add phase-one chat interface"
```

## Chunk 4: Pet Shell and Visible State

### Task 4: Build the floating orange-cat shell

**Files:**
- Create: `src/features/pet/PetShell.tsx`
- Create: `src/features/pet/PetState.ts`
- Create: `src/features/pet/SelfTalkBubble.tsx`

- [ ] **Step 1: Create a lightweight visual placeholder**

Start with:

- 2D cat shell
- idle state
- hover state
- busy state
- success state

- [ ] **Step 2: Add self-talk bubble**

Bubble examples:

- "让我看看"
- "我去打开它"
- "正在找一下"

- [ ] **Step 3: Bind shell state to task execution state**

When work is running:

- pet shows busy animation
- bubble updates with simple human-readable status

- [ ] **Step 4: Verify**

Expected:

- pet remains visible
- shell state changes are noticeable

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: add xixi pet shell and visible task states"
```

## Chunk 5: Personas and Model Routing

### Task 5: Add personas and task-based model routing

**Files:**
- Create: `src/features/personas/personas.ts`
- Create: `src/features/personas/PersonaSelector.tsx`
- Create: `src/features/models/modelRouter.ts`
- Create: `src/features/tasks/taskClassifier.ts`

- [ ] **Step 1: Define the initial persona set**

Include:

- companion
- calm professional
- playful cat
- focused operator

- [ ] **Step 2: Define model routing rules**

Initial routing config:

- conversation heavy -> conversational model
- coding or repo work -> coding-focused model
- analysis and planning -> reasoning route

- [ ] **Step 3: Define task classification result object**

It should include:

- task category
- risk level
- persona
- model route
- executor route

- [ ] **Step 4: Verify**

Feed sample phrases through the classifier manually:

- open QQ
- play a song
- analyze this idea
- write code for a tool

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: add personas and task routing"
```

## Chunk 6: Action Executor Core

### Task 6: Implement the safe executor framework

**Files:**
- Create: `src/features/executors/types.ts`
- Create: `src/features/executors/executorRegistry.ts`
- Create: `src/features/executors/appLauncher.ts`
- Create: `src/features/executors/browserExecutor.ts`
- Create: `src/features/executors/fileExecutor.ts`

- [ ] **Step 1: Define executor interface**

Every executor must accept:

- normalized task object
- context
- safety constraints

And return:

- success or failure
- user-readable summary
- optional machine-readable result

- [ ] **Step 2: Implement app launch executor**

Support known apps first.

Examples:

- QQ
- Chrome
- Edge
- local music player later if detectable

- [ ] **Step 3: Implement browser executor**

Phase 1 supported actions:

- open a URL
- search query
- open a known site

- [ ] **Step 4: Implement D-drive file executor**

Phase 1 supported actions:

- open file
- open folder
- list directory contents

- [ ] **Step 5: Enforce safety boundaries**

Allowed by default:

- browser actions
- D-drive inspection
- app launch

Blocked without confirmation:

- destructive file changes
- external account actions

- [ ] **Step 6: Verify**

Manual test each executor with sample inputs.

- [ ] **Step 7: Commit**

```bash
git add .
git commit -m "feat: add safe local executors for xixi"
```

## Chunk 7: Chat-to-Action Loop

### Task 7: Connect the chat window to the executor framework

**Files:**
- Modify: `src/features/chat/ChatWindow.tsx`
- Modify: `src/features/tasks/taskClassifier.ts`
- Modify: `src/features/executors/executorRegistry.ts`
- Create: `src/features/tasks/taskRunner.ts`

- [ ] **Step 1: Convert user message into a task**

On submit:

- classify request
- assign persona and route
- decide direct execute vs confirm

- [ ] **Step 2: Add small-task direct execution**

Examples:

- open app
- open folder
- browser search

- [ ] **Step 3: Add high-risk confirmation interaction**

Render a plain-language confirmation card in chat before proceeding.

- [ ] **Step 4: Add result messages**

The assistant must explain:

- what it understood
- what it did
- whether it succeeded

- [ ] **Step 5: Verify**

Test prompts:

- Open Chrome
- Open QQ
- Open D drive
- Search the weather today

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: connect chat requests to local action execution"
```

## Chunk 8: Skill and Specialist Extension Layer

### Task 8: Build the extension registry

**Files:**
- Create: `src/features/skills/skillManifest.ts`
- Create: `src/features/skills/skillRegistry.ts`
- Create: `src/features/skills/specialists.ts`

- [ ] **Step 1: Define skill manifest format**

Include:

- name
- role
- categories
- permissions
- model preference

- [ ] **Step 2: Register initial non-code specialists**

Initial set:

- brainstorming
- project analysis
- research synthesis
- UX review
- documentation

- [ ] **Step 3: Expose the registry to task routing**

The router should be able to ask:

- is there a specialist for this task?
- if yes, which one should shape the response?

- [ ] **Step 4: Verify**

Use sample tasks to confirm registry lookup works.

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: add skill and specialist registry"
```

## Chunk 9: Persistence, Logs, and Non-Coder Transparency

### Task 9: Add local persistence and plain-language traces

**Files:**
- Create: `src/shared/storage/sessionStore.ts`
- Create: `src/shared/storage/settingsStore.ts`
- Create: `src/features/tasks/taskLog.ts`

- [ ] **Step 1: Persist conversations locally**

- [ ] **Step 2: Persist persona and basic settings**

- [ ] **Step 3: Persist task history**

Task history should be plain-language, not just raw machine logs.

- [ ] **Step 4: Verify**

Restart app and confirm:

- session survives
- persona survives
- task records are readable

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: add local persistence and readable task history"
```

## Chunk 10: GitHub Collaboration Readiness

### Task 10: Prepare the repository for contributors

**Files:**
- Modify: `README.md`
- Create: `CONTRIBUTING.md`
- Create: `docs/notes/architecture-overview.md`
- Create: `.github/ISSUE_TEMPLATE/feature_request.md`
- Create: `.github/ISSUE_TEMPLATE/bug_report.md`

- [ ] **Step 1: Expand the README**

Add:

- project vision
- phase-one goals
- why it matters for non-coders
- contribution areas

- [ ] **Step 2: Add contribution guidance**

Target contributors across:

- Tauri
- React
- automation
- AI routing
- pet animation
- desktop UX

- [ ] **Step 3: Add issue templates**

- [ ] **Step 4: Verify**

Review repo as if you were a GitHub visitor with no project history.

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "docs: prepare xixi for open-source contributors"
```

## Chunk 11: Push and Repository Publication

### Task 11: Connect to GitHub and publish

**Files:**
- Modify: git remote configuration

- [ ] **Step 1: Check whether local GitHub credentials are already usable**

Run:

```bash
git remote -v
git config --global --get credential.helper
```

- [ ] **Step 2: If no remote exists, create one**

Preferred repository name:

- `xixi`

- [ ] **Step 3: Push main branch**

Run:

```bash
git push -u origin main
```

- [ ] **Step 4: Verify**

Confirm the remote repository shows:

- README
- design doc
- implementation history

- [ ] **Step 5: Commit any final sync changes**

```bash
git add .
git commit -m "chore: finalize initial publication state"
```

## Recommended Execution Order

1. Foundation and GitHub positioning
2. Desktop shell
3. Chat interface
4. Pet shell
5. Personas and model routing
6. Executor framework
7. Chat-to-action loop
8. Skill registry
9. Persistence
10. GitHub contributor polish
11. Remote publication

## Success Criteria

Phase 1 is successful when:

- xixi launches as a desktop app
- the pet shell is visible
- double-click opens the main chat window
- the chat feels modern and usable
- simple natural-language computer actions work
- small tasks execute directly
- high-risk tasks confirm first
- personas are selectable
- visible busy and self-talk states exist
- the project repository is attractive to contributors

## Notes

- Do not chase a perfect cat model first.
- Keep the executor surface narrow until trust is earned.
- Preserve extension points for future specialist packs and model routes.
- Keep every user-visible explanation understandable to a non-coder.
