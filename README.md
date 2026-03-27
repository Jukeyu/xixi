# xixi | Desktop Pet Assistant

`xixi` 是一个正在持续迭代的桌面宠物助手（Desktop Pet + Chat + Skills）。

我们的目标不是做“只会聊天”的窗口，而是做一个真正可执行、可审计、可扩展的本地助手：
- 通过聊天指令驱动桌面操作
- 通过技能系统连接真实能力
- 逐步走向无障碍辅助（帮助老年人和肢体不便用户更轻松使用电脑）

## 为什么做这个项目

很多人会被复杂软件界面劝退，尤其是老年用户、低视力用户、肢体不便用户。  
`xixi` 希望把“会操作电脑”这件事，变成“会说清楚需求”这件事。

长期愿景：
- Chat-to-Action：聊天即操作
- Voice-to-Action：后续接入语音指令
- Skill Marketplace：社区技能共建
- Assistive Computing：无障碍电脑助手

## 当前状态（As of 2026-03-27）

这是一个可运行的桌面应用原型，已具备真实可用链路：
- Tauri + React + TypeScript 桌面壳
- 命令解析 -> 风险分级 -> 本地执行 -> 结构化日志
- 支持托盘常驻、主窗体隐藏后宠物窗体显示
- 支持模型聊天模式（OpenAI-compatible `/chat/completions`）
- 支持本地技能扩展（JSON + Python/PowerShell）

## 能力矩阵（已实现 vs 规划中）

### 已实现（Real, Runnable）
- 打开网站、搜索网页、打开应用、打开目录
- 鼠标/键盘基础动作（点击、滚动、热键、输入）
- 人类化输入动作（平滑移动、拖拽、节奏输入）
- 屏幕观测技能：
  - `screen_watch_ocr.py`
  - `screen_intent_watch.py`
  - `screen_behavior_watch.py`
  - `latest screen summary`（融合意图+行为报告）
- 页面技能（Page-Agent 风格）：
  - `page agent inspect <url>`
  - `page agent click <url> <text>`
  - `latest page agent`

### 规划中（Roadmap）
- 语音输入与语音反馈链路
- 更强的多窗口任务拆解（Task Planning）
- 股票观察与交易辅助技能（先模拟，再实盘，严格风险门控）
- 社区技能目录与一键安装机制
- 无障碍体验专项优化（字体、对比度、低学习成本流程）

## 快速开始

```bash
cd apps/desktop
npm install
npm run tauri:dev
```

## 构建

```bash
cd apps/desktop
npm run tauri:build
```

Windows 构建产物：
- `apps/desktop/src-tauri/target/release/app.exe`
- `apps/desktop/src-tauri/target/release/bundle/msi/...`
- `apps/desktop/src-tauri/target/release/bundle/nsis/...`

## 测试与验证

```bash
cd apps/desktop
npm run test:smoke
```

`test:smoke` 会执行 TypeScript 检查、前端构建和 Rust 单元测试。

## 常用命令示例

```text
open site github.com
search web tauri tray icon
open app vscode
open folder downloads

screen intent coding
watch screen behavior workflow
latest screen intent
latest screen behavior
latest screen summary

page agent inspect example.com
page agent click example.com More information
latest page agent

human move 960,540
human click 920,520
human drag 760,420 to 1080,640
human type hello from xixi
```

## 本地技能系统

- 技能目录：`%LOCALAPPDATA%\xixi\skills`
- 脚本目录：`%LOCALAPPDATA%\xixi\skills\scripts`
- 运行日志：`%LOCALAPPDATA%\xixi\skills\runs`
- 运行命令：`run skill <skill_id> [input]`

更多说明见：
- `docs/skills/local-skills.md`
- `docs/skills/github-research-notes-2026-03-27.md`

## 依赖说明（Python）

OCR 与屏幕观测：
```bash
pip install mss pillow pytesseract
```

桌面输入：
```bash
pip install pyautogui
```

页面自动化：
```bash
pip install playwright
python -m playwright install chromium
```

## 安全边界（当前）

- 只执行已支持命令；不支持能力会明确拒绝
- 不做“假执行成功”反馈
- `run_script` 只允许技能脚本目录内 `.py` / `.ps1`
- 高风险操作需要确认（UI 风险分层）
- 每次动作写入结构化日志，便于审计与回溯

## 每次上传都要同步优化介绍页（必须执行）

每次 push / release，请同步更新 README 文案，至少完成以下 6 项：
1. 更新“当前状态”日期和能力快照
2. 写明本次新能力的用户价值（不是只写技术实现）
3. 区分“已实现”和“规划中”，禁止把规划写成已完成
4. 增加至少 1 条可直接复制运行的命令示例
5. 更新新增依赖与运行前提
6. 若有风险边界变化，必须更新“安全边界”章节

执行细则见：
- `docs/copywriting-playbook.md`

## 贡献方向（欢迎加入）

- 桌面技能执行器与可回放日志
- 屏幕理解与用户意图推断
- 语音交互链路
- 无障碍体验优化
- 风险控制与安全策略

如果你希望参与一个“能真正帮助人完成电脑操作”的开源项目，欢迎提 PR 或 Issue。

## 项目结构

- `apps/desktop`：Tauri + React 桌面应用
- `docs/skills`：技能文档与研究参考
- `ARCHITECTURE.md`：架构说明
- `ROADMAP.md`：路线图
- `CONTRIBUTING.md`：贡献规范

## License

See repository license and third-party dependency licenses before redistribution.

