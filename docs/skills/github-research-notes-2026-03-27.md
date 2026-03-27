# GitHub Skill Research Notes (2026-03-27)

This note records practical open-source options for:

- screen watching (capture + OCR)
- desktop actions (mouse/keyboard + Windows app control)
- screen intent inference (foreground app + OCR evidence)

## Selected Building Blocks

1. `python-mss` (fast screen capture, multi-monitor)  
   Repo: https://github.com/BoboTiG/python-mss  
   Docs: https://python-mss.readthedocs.io/  
   License: MIT  
   Decision: use in `screen_watch_ocr.py` for screenshot loop.

2. `pytesseract` + Tesseract OCR engine  
   Repo: https://github.com/madmaze/pytesseract  
   Tesseract docs: https://tesseract-ocr.github.io/  
   License: Apache-2.0 (wrapper)  
   Decision: use for OCR text extraction in keyword detection flow.

3. `PyAutoGUI` (cross-platform mouse/keyboard automation)  
   Docs: https://pyautogui.readthedocs.io/en/latest/  
   Repo: https://github.com/asweigart/pyautogui  
   License: BSD-3-Clause  
   Decision: use in `safe_desktop_action.py` with command blocklist and failsafe.

4. `pywinauto` (Windows UI automation)  
   Docs: https://pywinauto.readthedocs.io/en/latest/getting_started.html  
   Repo: https://github.com/pywinauto/pywinauto  
   License: BSD-3-Clause  
   Decision: keep as phase-next option for stronger control of native Windows controls.

5. Win32 foreground-window API references (primary source)  
   - GetForegroundWindow: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow  
   - GetWindowTextW: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextw  
   - GetWindowThreadProcessId: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid  
   - QueryFullProcessImageNameW: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew  
   Decision: use `ctypes` + Win32 APIs in `screen_intent_watch.py` to read active window title and process safely, without adding heavy native dependencies.

6. Windows UI Automation overview (primary source)  
   Docs: https://learn.microsoft.com/en-us/windows/win32/winauto/entry-uiauto-win32  
   Decision: keep as phase-next path for richer control-tree understanding beyond title/OCR heuristics.

## Evaluated but Not Adopted as Default

1. `Open Interpreter`  
   Repo: https://github.com/OpenInterpreter/open-interpreter  
   License: AGPL-3.0  
   Reason not default: copyleft obligations and broader runtime scope than current local-skill architecture.

2. `browser-use`  
   Repo: https://github.com/browser-use/browser-use  
   License: MIT  
   Reason not default: good for browser agents, but current phase focuses on local desktop skill scripts first.

3. `Microsoft UFO`  
   Repo: https://github.com/microsoft/UFO  
   License: MIT  
   Reason not default: architecture is much heavier than current local-script skill runtime; useful as future reference for multi-device orchestration patterns.

4. `page-agent`  
   Repo: https://github.com/alibaba/page-agent  
   License: MIT  
   Decision: adopt its "in-page agent" idea in lightweight form by adding `page_agent_web.py` (inspect interactive DOM elements and optional text-based click).

5. `AppAgent`  
   Repo: https://github.com/TencentQQGYLab/AppAgent  
   License: MIT  
   Decision: use as reference for perception-to-action decomposition and action trace logging style.

6. `osworld-human`  
   Repo: https://github.com/WukLab/osworld-human  
   License: MIT (repository level)  
   Decision: keep as benchmark reference for evaluating future real computer-use efficiency.

## Academic References Applied

1. ScreenAI (UI + visual understanding)  
   Paper: https://arxiv.org/abs/2402.04615  
   Applied takeaway: combine multiple visual signals (window context + OCR text) instead of relying on one cue.

2. SeeClick (GUI grounding)  
   Paper: https://arxiv.org/abs/2401.10935  
   Applied takeaway: keep action planning grounded in observed on-screen evidence and attach evidence fields in reports.

3. OSWorld benchmark (real computer environments)  
   Paper: https://arxiv.org/abs/2404.07972  
   Applied takeaway: prefer robust real-environment logging and explicit failure modes over fragile "demo-only" pipelines.

4. AppAgent (multimodal app interaction)  
   Paper: https://arxiv.org/abs/2312.13771  
   Applied takeaway: model behavior/intention as staged perception-to-action flow, with deterministic local safety checks before execution.

5. Mouse movement representation for attention prediction  
   Paper: https://arxiv.org/abs/2006.01644  
   Applied takeaway: add cursor trajectory features (distance, delta, switching patterns) in `screen_behavior_watch.py`.

6. Fitts' law (classic HCI motor model)  
   Source: https://pubmed.ncbi.nlm.nih.gov/13174710/  
   Applied takeaway: use distance-based movement-time estimation in `human_input_ops.py` for smoother human-like pointer movement.

## Integration Rules Used in xixi

1. Keep script execution inside `%LOCALAPPDATA%\\xixi\\skills\\scripts`.
2. Keep stdout/stderr logs under `%LOCALAPPDATA%\\xixi\\skills\\runs`.
3. Only allow `.py` and `.ps1` for now.
4. Add explicit risk levels per skill (`medium-risk`, `high-risk`).
5. Block known dangerous keyboard combos in default desktop action script.
6. For screen intent, infer only coarse intent classes by default (coding/research/trading/etc.); never auto-execute high-risk actions from inference alone.
