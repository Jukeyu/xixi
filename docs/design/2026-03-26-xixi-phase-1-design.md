# xixi Phase 1 Design

Date: 2026-03-26
Status: Draft
Project: xixi / 晰晰

## Product Summary

xixi is a desktop AI pet application for non-coders.

On the desktop, xixi appears as a small orange cat companion. Double-clicking the pet opens a maximized chat window similar to GPT or Doubao. The chat window is the main control center. Users speak in natural language. xixi understands intent, decides whether a task is small or high-risk, and then either executes immediately or asks for confirmation.

Phase 1 prioritizes real usefulness over visual perfection:

1. A real chat window that feels like a modern AI assistant
2. Real local computer actions from natural language
3. A lightweight pet shell that shows presence, self-talk, and busy state

## User Promise

This project should make AI automation understandable and usable for people who do not know how to code.

The user should be able to say things like:

- Open QQ
- Find and play a song
- Open Chrome and search for something
- Organize files in D drive
- Remind me later if the wind is strong today

And xixi should either do it directly or explain what it is doing in simple language.

## Phase 1 Scope

### Included

- Desktop pet shell with small orange cat placeholder appearance
- Double-click to open the main chat window
- Maximized chat interface
- Multi-persona switching in the chat interface
- Model routing by task type
- Natural-language task understanding
- Local action execution for browser and D drive workflows
- Basic desktop app launching
- Self-talk and busy-state feedback on the pet shell
- Small-task direct execution, large-task confirmation flow
- Extensible plugin or skill entry points
- GitHub-friendly project structure and open-source positioning

### Deferred

- High-fidelity 3D pet
- Full emotion engine
- Complex memory system across long time ranges
- Deep unattended night automation
- Full marketplace of third-party skills
- Highly polished voice interaction
- Advanced cross-app computer vision for every desktop app

## Product Principles

1. Usefulness first. The pet is not decoration. It must help.
2. Natural language first. The user should not need command syntax.
3. Visible intent. xixi should show what it is thinking and doing.
4. Gentle trust. Small tasks act directly. Bigger tasks confirm first.
5. Non-coder friendly. Every action and state should be understandable.
6. Extensible core. Skills, models, and specialist agents should plug in later without rewriting the app.

## Core Experience

### Desktop Layer

xixi lives on the desktop as a floating orange-cat companion.

Phase 1 visual behavior:

- idle state
- hover reaction
- busy state
- success reaction
- thinking or self-talk bubble

Phase 1 pet quality target:

- 2D or lightweight rendered shell first
- visually distinctive enough to feel like a product
- animation simple but expressive

### Chat Layer

Double-clicking xixi opens a maximized desktop chat window.

The chat window should feel close to a modern AI assistant:

- left rail or top bar for persona and session controls
- large scrolling conversation area
- clear input box
- attachment or action affordances later
- execution trace area kept simple for non-coders

### Task Execution Loop

1. User sends a natural-language request
2. xixi classifies the task
3. xixi chooses persona and model routing
4. xixi decides if the task is small or high-risk
5. xixi executes or asks for confirmation
6. xixi reports progress in chat
7. xixi pet shows visible busy behavior
8. xixi returns result and next-step suggestion

## Small vs High-Risk Task Policy

### Small tasks

Execute directly:

- open a known app
- open a browser
- search the web
- open a file in D drive
- play media
- collect public information
- inspect a webpage

### High-risk tasks

Require confirmation:

- delete or overwrite files
- move large groups of files
- send messages or emails
- install or uninstall software
- system configuration changes
- account or payment actions

## Personas

Phase 1 supports multiple personas, but all personas share one execution core.

Initial persona model:

- default companion persona
- calm professional persona
- playful cat persona
- focused operator persona

Personas affect:

- tone
- self-talk style
- reminder language
- response formatting

Personas do not affect:

- safety rules
- execution permissions
- system boundaries

## Model Routing

Phase 1 should be designed for automatic model routing by task type.

Recommended routing concept:

- conversational and personality-heavy replies -> flagship conversational model
- code generation and repo work -> coding-focused model
- planning or analysis -> reasoning-oriented model
- lightweight classification -> cheaper faster model later

The app should not hardcode one single model path into the whole experience.

Instead, it should expose a `Task Router` that chooses:

- persona
- model family
- specialist agent or skill
- executor type

## System Architecture

Phase 1 recommended architecture uses five modules.

### 1. Desktop Shell

Responsibilities:

- floating pet window
- tray integration later
- click and drag behavior
- double-click open
- visual states
- small speech bubbles

### 2. Main Chat App

Responsibilities:

- maximized chat interface
- session history
- persona switching
- confirmation prompts
- progress and result rendering

### 3. Intelligence Orchestrator

Responsibilities:

- understand user intent
- classify task type
- route to model and specialist logic
- determine safety level
- generate user-facing plan in plain language

### 4. Action Executor

Responsibilities:

- app launch
- browser actions
- D drive file actions
- scripted desktop tasks
- executor result reporting

Phase 1 executor backends:

- browser automation backend
- desktop scripting backend
- file-operation backend

### 5. Skill and Agent Registry

Responsibilities:

- install entry points for future skills
- attach specialist prompt packs
- map task types to specialist assistants
- make future non-code agents pluggable

## Technology Recommendation

Recommended stack for Phase 1:

- Desktop shell and main app: Tauri + web frontend
- Frontend UI: React
- State management: simple app-level store first
- Local backend and orchestration: Rust commands plus a local service layer
- Desktop action backends: integrate Power Automate Desktop, AutoHotkey, and browser automation through controlled adapters
- Storage: local SQLite or lightweight structured local storage

Why this direction:

- Tauri gives a real desktop app feel without the heavier Electron footprint
- React is strong for building a GPT-like chat experience quickly
- Tauri also keeps room for a lightweight floating shell and native bindings
- Local adapters let xixi grow from browser and D drive actions into richer automation later

## Why Not Make the Pet the Whole App

The pet should be the emotional entry point, not the entire application surface.

If the pet tries to be the main place for all interaction, the product becomes cute but cramped. The real power of xixi is the maximized chat workspace plus visible execution.

So Phase 1 treats the pet as:

- presence
- notification
- emotional layer
- quick entry point

And treats the chat window as:

- main intelligence console
- task authoring area
- explanation and result surface

## Plugin and Skill Architecture

Phase 1 must leave an install path for future skills and agent packs.

Recommended registry shape:

- skill manifest
- specialist manifest
- task categories
- model preferences
- capability permissions

Examples of non-code skill packs to support later:

- brainstorming
- project analysis
- product strategy
- UX review
- documentation
- research synthesis
- reminder and life-assistant packs

Examples of code-oriented packs later:

- coding
- repo review
- test generation
- debugging

## Open-Source Positioning

The GitHub positioning should explicitly welcome both non-coders and expert contributors.

Core message:

xixi is an AI automation desktop companion for people who do not know how to code. It turns natural language into usable desktop automation and makes advanced AI workflows approachable through a pet-like interface.

Contributor appeal:

- desktop app architecture
- AI chat UX
- local automation
- model routing
- plugin systems
- pet animation
- Live2D or 3D shell evolution
- safety and trust UX

## Biggest Risks

### 1. Overbuilding too early

Risk:

Trying to make the pet, chat, model system, execution system, and reminders all perfect at once.

Mitigation:

Phase 1 optimizes for a working core loop, not total polish.

### 2. Desktop action reliability

Risk:

Natural-language intent may not map cleanly to local desktop actions.

Mitigation:

Start with narrow, reliable action categories:

- launch known apps
- browser actions
- D drive operations

### 3. Trust breakdown

Risk:

If xixi acts unpredictably, users will stop trusting it.

Mitigation:

Use the small-task versus high-risk policy from day one, and show visible execution state.

### 4. Personality interfering with clarity

Risk:

Cute cat behavior could make the app feel unserious or confusing.

Mitigation:

Keep personality in tone and shell animation, but keep execution messages clear.

## Recommended Phase 1 Outcome

At the end of Phase 1, the user should be able to:

1. Launch xixi as a real desktop app
2. See a small orange cat on the desktop
3. Double-click to open a modern maximized chat window
4. Chat naturally with different personas
5. Ask xixi to open apps, open websites, do browser tasks, and handle simple D drive tasks
6. Watch xixi show self-talk and busy feedback while acting
7. Trust that bigger actions will ask before proceeding

## Recommendation

Proceed with an ability-first Phase 1:

- real desktop shell
- real chat window
- real action execution
- lightweight cat shell
- extensible model and skill routing

This gives xixi a living core immediately, while preserving a strong path toward the fuller pet-operating-system vision.
