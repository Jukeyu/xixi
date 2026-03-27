# Computer-Use Landscape for xixi (2026-03-28)

## Scope

This note focuses on **2025-2026** open-source projects and papers that are directly relevant to:

- desktop computer-use
- screen understanding
- GUI automation
- browser/GUI hybrids that can feed into a desktop assistant such as `xixi`

I prioritized **primary sources only**:

- official GitHub repositories
- official project pages
- arXiv papers

`page-agent` is treated as an anchor because it represents a strong **DOM-first / text-first** alternative to screenshot-heavy GUI agents.

## Short Takeaways

1. There is no single best stack for all tasks.
   - Web tasks are increasingly handled by **DOM-first** systems such as `page-agent` and `browser-use`.
   - Native desktop tasks still need a **screen-grounding + input execution** stack.

2. The strongest open systems separate:
   - planning
   - grounding / perception
   - action execution
   - memory / recovery

3. For `xixi`, the best near-term direction is **hybrid**:
   - keep deterministic local skills for execution
   - add a stronger **screen parser / grounding layer**
   - add a **safe skill-selection layer**
   - keep browser automation on a separate lane from desktop-global control

## Open-Source Projects

| Project | Main use | Stack / execution model | What xixi can borrow directly | Integration risk for xixi | Primary sources |
|---|---|---|---|---|---|
| [Alibaba Page Agent](https://github.com/alibaba/page-agent) | In-page natural-language control for web UIs | TypeScript, in-page JavaScript, DOM/text manipulation, optional Chrome extension, optional MCP server | Very strong pattern for **browser-only tasks**: prefer DOM and text selectors before screenshots or coordinates; useful for a future `xixi` browser lane | High scope mismatch for native desktop: Page Agent is fundamentally **web-first**, not Windows-native. It will not solve native app control by itself | GitHub repo |
| [ByteDance UI-TARS-desktop / Agent TARS](https://github.com/bytedance/UI-TARS-desktop) | Native GUI agent stack for local/remote computer and browser operators | Multimodal GUI-agent stack, desktop app, local and remote operators, browser operators, model-driven control | Borrow the **operator abstraction**: separate local computer operator, remote computer operator, and browser operator; also useful as a reference for multi-provider model adapters and GUI-agent UX | Heavy model/runtime complexity; more agentic and less deterministic than `xixi`'s current local-skill approach; risk of making the product harder to debug and safer execution harder to guarantee | GitHub repo, UI-TARS paper link from repo |
| [Microsoft OmniParser + OmniTool](https://github.com/microsoft/OmniParser) | Screenshot parsing into structured UI elements for vision-based GUI agents | Python, screenshot parsing, checkpoint-based models, `omnitool` for Windows 11 VM control, integration with external vision / LLM models | Borrow the **screen-to-structured-elements** layer. This is one of the most useful additions for `xixi` if you want it to understand what is on screen before acting | Vision-only grounding is still probabilistic; latency and GPU cost can be high; license is **CC-BY-4.0**, so downstream product/legal handling should be reviewed carefully | GitHub repo |
| [Simular Agent-S / gui-agents](https://github.com/simular-ai/Agent-S) | Full computer-use agent framework across Windows / macOS / Linux | Python, multi-agent planning, grounding model + main model separation, OCR, optional local coding environment | Borrow the **planner vs grounding split**, reflection/failure-recovery patterns, and benchmark-oriented architecture; strong reference for long-horizon desktop tasks | Operational complexity is high; the repo explicitly warns about local code execution risk; current design assumptions such as **single-monitor** setups may not fit all `xixi` users | GitHub repo, Agent S2 paper |
| [browser-use](https://github.com/browser-use/browser-use) | Browser agents with programmable automation, custom tools, and persistent browser state | Python 3.11+, library + CLI, browser session control, open benchmark, custom tools | Borrow the **browser lane abstraction**, persistent browser sessions, and testable CLI/state model. Strong candidate if `xixi` needs browser workflows without mixing them into desktop-global control | It is browser-first, not desktop-first. Also, the project explicitly positions open source vs cloud separately; the strongest production features are not all in OSS | GitHub repo |
| [PC-Agent-E](https://github.com/GAIR-NLP/PC-Agent-E) | Training and deployment framework for computer-use agents | Python, trajectory collection, thought completion, trajectory boosting, agent training, WindowsAgentArena-V2 | Borrow the **training-data pipeline ideas**: trajectory logging, post-processing, data augmentation, and eventually offline improvement of `xixi`'s skills and policy | More useful as a **research/training** reference than a drop-in runtime. It does not directly replace a production-safe deterministic executor | GitHub repo, paper |

## Papers

| Paper | Why it matters | What xixi can borrow | Main risk if adopted too literally | Primary source |
|---|---|---|---|---|
| [UI-TARS: Pioneering Automated GUI Interaction with Native Agents (2025)](https://arxiv.org/abs/2501.12326) | A strong reference point for native GUI-agent design and multimodal control | Borrow the idea of a **native-agent interface** and a model/grounding layer designed specifically for GUI tasks | Pushing too far toward end-to-end agent control can reduce determinism and auditability | arXiv |
| [Agent S2: A Compositional Generalist-Specialist Framework for Computer Use Agents (2025)](https://arxiv.org/abs/2504.00906) | Important because it explicitly separates specialist capabilities and generalist orchestration | Borrow the **specialist routing** idea for `xixi`: browser, OCR, local skills, file tasks, and recovery should not all be one monolith | Composition adds coordination overhead. If done too early, product complexity grows faster than reliability | arXiv, Agent-S repo |
| [Efficient Agent Training for Computer Use / PC-Agent-E (2025, ICLR 2026)](https://arxiv.org/abs/2505.13909) | Useful if `xixi` eventually learns from trajectories rather than only hand-written skills | Borrow the **trajectory collection and post-processing** pipeline, not just the final trained agent | Training pipelines can distract from near-term product value; data quality and privacy become major concerns | arXiv, PC-Agent-E repo |
| [CUA-Skill: Develop Skills for Computer Using Agent (2026)](https://arxiv.org/abs/2601.21123) | One of the clearest 2026 directions: represent computer-use knowledge as reusable skills with execution/composition graphs | Very relevant to `xixi`: borrow **skill cells**, parameterized execution graphs, and memory-aware failure recovery. This aligns well with `xixi`'s local skill philosophy | If copied naively, it may push too much abstraction before the underlying execution substrate is stable enough | arXiv, [project page](https://microsoft.github.io/cua_skill/) |
| [A Survey on (M)LLM-Based GUI Agents (2025)](https://arxiv.org/abs/2504.13865) | Strong synthesis of the field across perception, exploration, planning, and interaction | Use it as an architectural checklist when deciding whether a new feature belongs to perception, memory, planning, or execution | Survey guidance is broad; it does not reduce engineering effort by itself | arXiv |
| [Towards Trustworthy GUI Agents: A Survey (2025)](https://arxiv.org/abs/2503.23434) | Important for `xixi` because safety, privacy, and controllability are product requirements, not optional extras | Borrow trustworthiness dimensions for acceptance criteria: **security, reliability, transparency, ethics, evaluation** | The risk is under-scoping safety and shipping unsafe autonomy before recovery/visibility is in place | arXiv |

## What Is Most Reusable for xixi Right Now

### 1. Page Agent's DOM-first design

Best use inside `xixi`:

- browser workflows
- browser forms
- web admin panels
- web accessibility features

Why it matters:

- It avoids screenshot-only control when the DOM already exposes structure.
- It is lighter and more deterministic than full screenshot-driven browser agents.

Practical recommendation:

- For browser tasks, `xixi` should prefer **DOM/text control first**, then fall back to browser automation libraries, and only use global desktop clicking last.

### 2. OmniParser's screen-to-structure layer

Best use inside `xixi`:

- native desktop screen understanding
- "what is on my screen?" summaries
- better target grounding before mouse/keyboard actions

Why it matters:

- `xixi` already has OCR and screen-watch scripts.
- What is still missing is a stronger **UI-element grounding layer** that turns screenshots into structured actionable objects.

Practical recommendation:

- Add a new optional perception lane in `xixi`:
  - `capture screenshot`
  - `parse UI elements`
  - `attach confidence + bounding boxes`
  - only then choose action

### 3. Agent-S and CUA-Skill style skill composition

Best use inside `xixi`:

- safe multi-step local workflows
- reusable "micro-skills"
- failure recovery after a wrong click or stale screen state

Why it matters:

- `xixi` is already skill-oriented.
- The next step is not "make it fully autonomous everywhere".
- The next step is "make the existing skill system composable and state-aware".

Practical recommendation:

- Define a stricter skill interface:
  - preconditions
  - perception requirements
  - action kind
  - rollback/recovery hint
  - expected postcondition

### 4. Browser-use as an engineering reference, not the desktop core

Best use inside `xixi`:

- browser-only task lane
- reproducible browser command execution
- CLI-like automation/testing patterns

Why it matters:

- It has a good separation between agent, browser state, custom tools, and benchmarked tasks.

Practical recommendation:

- Do not merge browser-use concepts into all desktop tasks.
- Keep browser automation as a separate subsystem.

## Recommended Architecture Direction for xixi

### Near-term (best fit)

1. Keep `xixi`'s current deterministic local skills and action logs.
2. Add a **structured perception layer** inspired by OmniParser.
3. Add **safe skill composition** inspired by CUA-Skill and Agent-S2.
4. Keep browser tasks on a **DOM-first lane** inspired by Page Agent.

### Mid-term

1. Add a replayable trajectory format:
   - screenshot
   - parsed UI objects
   - chosen action
   - result
   - recovery path
2. Use those trajectories to build your own `xixi`-specific dataset.
3. Evaluate against a Windows-style benchmark subset before increasing autonomy.

### Not recommended yet

- Fully replacing deterministic skills with an end-to-end multimodal agent
- Letting arbitrary generated code drive local execution by default
- Treating browser automation and desktop automation as the same problem

## Suggested Next Borrowing Order

1. **Page Agent ideas** for browser-only workflows
2. **OmniParser-like structured screen parsing** for desktop perception
3. **CUA-Skill / Agent-S2 skill composition** for safe multi-step actions
4. **PC-Agent-E style trajectory tooling** once enough real usage data exists

## Notes on Source Quality

- Release and publication dates are taken from the linked primary sources.
- When I describe a project as "active in 2025-2026", that is an inference from repo updates, release notes, or paper/project-page timing.

## Primary Sources

### Projects

- Page Agent repo: https://github.com/alibaba/page-agent
- UI-TARS-desktop / Agent TARS repo: https://github.com/bytedance/UI-TARS-desktop
- OmniParser repo: https://github.com/microsoft/OmniParser
- Agent-S repo: https://github.com/simular-ai/Agent-S
- browser-use repo: https://github.com/browser-use/browser-use
- PC-Agent-E repo: https://github.com/GAIR-NLP/PC-Agent-E

### Papers

- UI-TARS (2025): https://arxiv.org/abs/2501.12326
- Agent S2 (2025): https://arxiv.org/abs/2504.00906
- Efficient Agent Training for Computer Use / PC-Agent-E (2025): https://arxiv.org/abs/2505.13909
- CUA-Skill (2026): https://arxiv.org/abs/2601.21123
- A Survey on (M)LLM-Based GUI Agents (2025): https://arxiv.org/abs/2504.13865
- Towards Trustworthy GUI Agents: A Survey (2025): https://arxiv.org/abs/2503.23434
