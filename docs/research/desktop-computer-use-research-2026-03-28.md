# Desktop Computer-Use / Screen-Understanding Research (Windows xixi, 2026-03-28)

## Scope
This brief focuses on practical, near-term computer-use and screen-understanding skills for the existing xixi desktop stack (Tauri/Rust orchestrator + local script skills), with Windows-first implementation choices backed by primary sources.

## Key Findings From Primary Sources

### 1) Windows control should be accessibility-first, not pixel-first
- Microsoft UI Automation (UIA) is the official accessibility and automation interface for Windows desktop UI; it exposes control patterns, properties, and tree structure that are more stable than image-only clicking.
- Microsoft explicitly documents UIA for automated testing scenarios, reinforcing it as the reliable path for structured UI interaction.
- Practical implication for xixi: use UIA element targeting first; only fall back to coordinates when UIA metadata is unavailable.

### 2) Keep synthetic input as controlled fallback
- Win32 `SendInput` is the canonical API for keyboard and mouse event injection on Windows.
- PyAutoGUI remains useful for coordinate fallback and includes built-in fail-safe behavior (mouse-to-corner abort), which is important for user safety.
- Practical implication for xixi: keep a strict "UIA-first, SendInput/PyAutoGUI-fallback" policy and require stronger confirmation for fallback actions.

### 3) Screen understanding should be layered: capture -> OCR -> grounding
- `python-mss` is a high-performance screenshot backend commonly used for real-time capture loops.
- Tesseract is a mature OCR engine with active documentation and model support; `pytesseract` provides a straightforward Python bridge.
- Practical implication for xixi: first implement deterministic capture+OCR for text-heavy tasks, then add optional multimodal grounding only where OCR and UIA coverage fail.

### 4) Browser tasks should use a browser-native automation lane
- Playwright's locator model and auto-waiting reduce fragility versus raw coordinate clicking in web contexts.
- Practical implication for xixi: route browser-domain tasks to Playwright skills rather than desktop-global mouse scripts whenever possible.

### 5) Benchmark and reference ecosystem points to hybrid agents
- OSWorld provides a broad benchmark for real computer-use agents and highlights current difficulty of open-ended desktop tasks.
- WindowsAgentArena focuses specifically on Windows with UI understanding and action grounding trajectories.
- Agent-S and page-agent repos are useful implementation references for planning/action loops and GUI grounding patterns.
- Practical implication for xixi: validate with small internal task suites inspired by OSWorld and WindowsAgentArena task styles.

### 6) Avoid over-investing in stale components
- WinAppDriver is still useful for some UI test compatibility flows, but its official repo shows long stagnation; it should not be the core future path.
- Practical implication for xixi: prioritize UIA plus modern agent loops over WinAppDriver-centric architecture.

## Practical Integration Plan For xixi (Windows)
1. Add a `desktop_perceive` skill contract returning JSON:
   - `active_window`, `ui_elements[]` (if accessible), `ocr_blocks[]`, `confidence`.
2. Implement action policy tiers in the Rust planner:
   - Tier A: UIA element actions (default).
   - Tier B: browser actions via Playwright.
   - Tier C: coordinate and input fallback with explicit safety gate.
3. Standardize run logs:
   - save screenshot path, OCR summary, chosen action type (`uia`/`browser`/`coordinate`), and confidence for replay/debug.
4. Add "human interrupt" invariants:
   - global hotkey stop, corner fail-safe, and per-step timeout.
5. Build a 20-50 task regression pack:
   - open app, switch window, fill form, extract on-screen text, browser search, copy/paste workflow.

## Recommended Stack (Short)
- Orchestration: existing xixi Rust planner/executor in Tauri.
- Windows structured control: `pywinauto` (UIA backend).
- Fallback input: Win32 `SendInput` plus PyAutoGUI fail-safe.
- Screen capture: `python-mss`.
- OCR: Tesseract 5 plus `pytesseract`.
- Web automation lane: Playwright (Python).
- Evaluation: OSWorld-inspired tasks plus WindowsAgentArena-style Windows scenarios.

## Primary Sources
- Microsoft UI Automation Overview: https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-uiautomationoverview
- Microsoft UI Automation for Automated Testing: https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-usefortesting
- Win32 `SendInput`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput
- pywinauto docs: https://pywinauto.readthedocs.io/en/latest/
- PyAutoGUI quickstart (fail-safe): https://pyautogui.readthedocs.io/en/latest/quickstart.html
- python-mss repo/docs: https://github.com/BoboTiG/python-mss
- Tesseract OCR docs: https://tesseract-ocr.github.io/tessdoc/
- pytesseract repo: https://github.com/madmaze/pytesseract
- Playwright Python locators: https://playwright.dev/python/docs/locators
- OSWorld benchmark paper: https://arxiv.org/abs/2404.07972
- OSWorld repo: https://github.com/xlang-ai/OSWorld
- WindowsAgentArena: https://microsoft.github.io/WindowsAgentArena/
- Agent-S repo: https://github.com/simular-ai/Agent-S
- page-agent repo: https://github.com/alibaba/page-agent
- WinAppDriver repo (staleness reference): https://github.com/microsoft/WinAppDriver
