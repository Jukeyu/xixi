# xixi / 晰晰

`xixi` 是一个开源的**桌面宠物应用程序**。  
它的目标不是做一个只会聊天的壳，而是逐步成长为一个能通过聊天或语音，帮助用户在电脑上完成真实操作的助手。

## 一句话定位

一个以“桌面宠物”形态呈现的 AI 助手：可聊天、可执行技能、可安全操作电脑。

## 我们在做什么

我们希望 `xixi` 最终具备这条能力链路：

1. 启动应用后可配置模型 API（对话框/设置面板）。
2. 用户通过聊天或语音下达任务。
3. `xixi` 将任务分解为可审计的技能步骤。
4. 在安全边界内执行真实电脑动作，并反馈结果。

## 使命与愿景

### 使命

把“能操作电脑的 AI”做成普通人可用、可理解、可托付的产品。

### 愿景

让老年人、上肢行动不便用户、非技术用户，都能通过自然语言完成日常电脑任务，而不是被复杂界面阻挡。

## 当前状态（真实能力，2026-03-27）

目前仓库是**可运行的 Phase 2 原型**，已实现：

- Windows 桌面应用（Tauri + React）
- 聊天输入与真实命令规划
- 白名单本地动作执行（文件夹、网站、应用启动等）
- 参数化指令（例如 `open site <domain>`、`search web <query>`）
- 本地技能系统（`run skill <id> [input]`，支持自己写技能 JSON）
- 托盘驻留（关闭窗口隐藏到托盘）
- 结构化执行日志与失败重试
- 明确拒绝未实现指令（不做“伪成功”）

## 还未完成（但正在推进）

以下能力是目标方向，当前版本尚未全部实现：

- 模型 API 填写与多模型切换面板
- 技能注册中心（安装/卸载/权限管理）
- 语音输入与语音指令执行链路
- 更完整的风险分级与高风险操作二次确认

## 长期方向（含你提到的高级场景）

我们会把高级能力放在**安全与合规优先**前提下推进：

- 电脑自主操作（聊天/语音驱动）
- 任务型技能生态（可扩展）
- 股票观察与交易辅助（研究与策略建议）
- 交易执行仅在明确授权、风控策略、可追溯日志下进行

## 为什么值得加入

这是一个技术价值和社会价值同时很高的方向：

- 技术上：LLM + 桌面自动化 + 技能系统 + 安全工程
- 产品上：桌面宠物形态、长期可用而非演示
- 社会上：面向老年人与行动受限人群的真实可访问性工具

如果你希望做一个“不是花架子、真的能帮到人”的 AI 项目，欢迎一起推进。

## Quick Start

```bash
cd apps/desktop
npm install
npm run tauri:dev
```

## Build

```bash
cd apps/desktop
npm run tauri:build
```

构建产物：

- `apps/desktop/src-tauri/target/release/app.exe`
- `apps/desktop/src-tauri/target/release/bundle/msi/...`
- `apps/desktop/src-tauri/target/release/bundle/nsis/...`

## Test Workflow

```bash
cd apps/desktop
npm run lint
npm run test:smoke
```

## Model API Chat Mode (OpenAI-compatible)

You can switch from command mode to model chat mode in Settings.

- Fill: `Model API base URL`, `Model name`, `API key`
- Example base URL: `https://api.openai.com/v1`
- In model mode, chat messages are sent to `/chat/completions`
- In command mode, xixi keeps using local action planning/execution

## Permission Profiles

- `Safe`: allow web + folder actions only
- `Balanced`: allow apps and scripts, but block high-risk actions
- `Advanced`: allow all current actions (high-risk still requires manual confirmation)

## 自定义技能（你可以自己写）

`xixi` 会从本地技能目录加载 JSON 技能文件，你可以自行扩展简单电脑操作。

- 技能目录（Windows）：`%LOCALAPPDATA%\xixi\skills`
- 脚本目录（代码技能）：`%LOCALAPPDATA%\xixi\skills\scripts`
- 脚本运行日志：`%LOCALAPPDATA%\xixi\skills\runs`
- 命令格式：`run skill <skill_id> [input]`
- 示例：`run skill open_github`、`run skill search_stock_news tsla`、`run skill screen_watch_ocr keyword=stock duration=20`、`run skill desktop_action_safe click`
- `high-risk` 技能会在 UI 中触发二次确认（不会自动执行）

技能格式与示例见：

- `docs/skills/local-skills.md`
- `docs/skills/github-research-notes-2026-03-27.md`

## Repo Structure

- `apps/desktop`: Tauri + React 桌面应用
- `docs/design`: 设计记录
- `docs/plans`: 实施计划
- `.github/workflows`: CI
- `ARCHITECTURE.md`: 当前架构说明
- `ROADMAP.md`: 里程碑路线图
- `CONTRIBUTING.md`: 协作说明

## Collaboration

欢迎提 Issue / PR，尤其是以下方向：

- 桌面技能执行器扩展（真实动作适配）
- 多语言与自然语言理解（中英文混合场景）
- 可访问性（大字体、低视力、老年友好、语音辅助）
- 风险控制与审计日志
- 语音交互与技能编排
