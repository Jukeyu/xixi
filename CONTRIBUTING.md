# Contributing to xixi

Thanks for helping build `xixi`.

The project values real, testable progress over demo-only features.
If a behavior cannot be executed or verified, we do not present it as completed.

## Principles

- Keep execution honest: no fake success states.
- Prefer small, verifiable changes.
- Add or update tests whenever behavior changes.
- Keep user-facing text clear for beginners.

## Local Setup

```bash
cd apps/desktop
npm install
npm run tauri:dev
```

## Validation Before PR

Run this before opening a pull request:

```bash
cd apps/desktop
npm run test:smoke
```

## What to Work On

High-value contribution themes:

1. New desktop executors
2. Better natural-language command mapping
3. Safety and confirmation policies
4. Skill and agent extension system
5. UI readability and accessibility

## PR Expectations

- Describe what is now real and executable.
- List exact commands used for verification.
- Include screenshots for UI changes.
- Mention any known limits or follow-up work.

## README Update Rule (Required)

For this project, each meaningful push should update the GitHub introduction page (`README.md`) together with code changes.
Use `docs/copywriting-playbook.md` as the required checklist and style baseline.
CI enforces this via `.github/workflows/ci.yml` + `scripts/check-readme-sync.mjs` for product-facing desktop changes.

Before opening a PR, verify:

1. README "current status" matches what is actually runnable.
2. New capabilities are reflected with user-facing command examples.
3. Unimplemented ideas are not described as finished features.
4. Dependency/setup changes are documented.
5. Safety boundaries and known limits remain explicit.
