# GitHub Skill Research Notes (2026-03-27)

This note records practical open-source options for:

- screen watching (capture + OCR)
- desktop actions (mouse/keyboard + Windows app control)

## Selected Building Blocks

1. `python-mss` (fast screen capture, multi-monitor)  
   Repo: https://github.com/BoboTiG/python-mss  
   License: MIT  
   Decision: use in `screen_watch_ocr.py` for screenshot loop.

2. `pytesseract` + Tesseract OCR engine  
   Repo: https://github.com/madmaze/pytesseract  
   License: Apache-2.0 (wrapper)  
   Decision: use for OCR text extraction in keyword detection flow.

3. `PyAutoGUI` (cross-platform mouse/keyboard automation)  
   Repo: https://github.com/asweigart/pyautogui  
   License: BSD-3-Clause  
   Decision: use in `safe_desktop_action.py` with command blocklist and failsafe.

4. `pywinauto` (Windows UI automation)  
   Repo: https://github.com/pywinauto/pywinauto  
   License: BSD-3-Clause  
   Decision: keep as phase-next option for stronger control of native Windows controls.

## Evaluated but Not Adopted as Default

1. `Open Interpreter`  
   Repo: https://github.com/OpenInterpreter/open-interpreter  
   License: AGPL-3.0  
   Reason not default: copyleft obligations and broader runtime scope than current local-skill architecture.

2. `browser-use`  
   Repo: https://github.com/browser-use/browser-use  
   License: MIT  
   Reason not default: good for browser agents, but current phase focuses on local desktop skill scripts first.

## Integration Rules Used in xixi

1. Keep script execution inside `%LOCALAPPDATA%\\xixi\\skills\\scripts`.
2. Keep stdout/stderr logs under `%LOCALAPPDATA%\\xixi\\skills\\runs`.
3. Only allow `.py` and `.ps1` for now.
4. Add explicit risk levels per skill (`medium-risk`, `high-risk`).
5. Block known dangerous keyboard combos in default desktop action script.
