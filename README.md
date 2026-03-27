# xixi · Desktop Pet Assistant（桌面宠物助手）

`xixi` 是一个正在持续迭代的开源桌面宠物应用。  
目标不是做“只会回复文字的聊天壳”，而是做一个可执行、可审计、可持续进化的本地助手：通过聊天命令（后续扩展语音）帮助用户在电脑上完成真实操作。

## 项目使命

- 让普通用户也能通过自然语言使用电脑功能，而不是被复杂界面卡住。
- 让老年人和上肢行动不便用户，逐步拥有“低门槛、可理解、可控”的数字助手。
- 在可用性和安全性之间做工程化平衡：只暴露真实能力，不伪造执行成功。

## 当前状态（As of 2026-03-27）

这是一个可运行的桌面应用原型，核心链路已打通：

- 桌面端：Tauri + React + TypeScript
- 真实动作链路：命令规划 → 风险评估 → 本地执行 → 结果日志
- 高风险动作默认需要人工确认
- 支持托盘常驻、宠物窗体、多姿态显示
- 支持本地技能扩展（JSON + Python/PowerShell）

## 真实能力清单（Only Real Features）

### 1) 桌面交互能力

- 打开网站、网页搜索、打开应用、打开文件夹
- 鼠标移动/点击/双击/右键/滚动
- 键盘输入、按键、组合键
- 关闭主窗口后进入托盘与宠物模式

### 2) 屏幕理解与意图观察能力

- `screen_watch_ocr.py`：盯屏 OCR 关键词检测
- `screen_intent_watch.py`：基于前台窗口 + OCR 的意图推断
- `screen_behavior_watch.py`：基于鼠标轨迹 + 屏幕动态的行为态推断
- `latest screen intent`：读取最近一次意图观察报告
- `latest screen behavior`：读取最近一次行为观察报告

### 3) 人类化输入能力

- `human_input_ops.py`：平滑移动、点击、拖拽、输入节奏模拟
- 支持命令：
  - `human move <x,y>`
  - `human click <x,y>`
  - `human drag <x1,y1> to <x2,y2>`
  - `human type <text>`

### 4) Page-Agent 风格网页技能（新）

- `page_agent_web.py`：本地浏览器自动化的轻量网页代理
- `page agent inspect <url>`：读取页面可交互元素
- `page agent click <url> <text>`：按文本尝试点击页面元素
- `latest page agent`：读取最近一次 page-agent 执行报告

### 5) 模型聊天模式

- 支持 OpenAI-compatible `/chat/completions`
- 双模式切换：命令模式 / 模型聊天模式
- 增强错误诊断：可识别 HTML 网关错误（如 Cloudflare 400）并提示修复方向

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

构建产物：

- `apps/desktop/src-tauri/target/release/app.exe`
- `apps/desktop/src-tauri/target/release/bundle/msi/...`
- `apps/desktop/src-tauri/target/release/bundle/nsis/...`

## 测试与验证

```bash
cd apps/desktop
npm run check
npm run lint
npm run test:smoke
```

`test:smoke` 会执行 `check + build + Rust tests`，用于保证核心能力可运行。

## 常用命令示例

```text
open site github.com
search web tauri tray icon
open app vscode
open folder downloads
watch screen stock
screen intent coding
watch screen behavior workflow
latest screen intent
latest screen behavior
page agent inspect example.com
page agent click example.com More information
latest page agent
human move 960,540
human click 960,540
human drag 760,420 to 1080,640
human type hello from xixi
```

## 本地技能系统

技能目录（Windows）：

- `%LOCALAPPDATA%\xixi\skills`
- `%LOCALAPPDATA%\xixi\skills\scripts`
- `%LOCALAPPDATA%\xixi\skills\runs`

技能命令：

```text
run skill <skill_id> [input]
```

技能文档：

- `docs/skills/local-skills.md`
- `docs/skills/github-research-notes-2026-03-27.md`

## Python 依赖

### OCR / 屏幕观察

```bash
pip install mss pillow pytesseract
```

### 输入自动化

```bash
pip install pyautogui
```

### 网页代理技能

```bash
pip install playwright
python -m playwright install chromium
```

## 安全边界（当前实现）

- 仅执行白名单动作，不支持的命令会明确拒绝
- 不提供“伪执行成功”反馈
- `run_script` 限制在本地技能脚本目录
- 仅允许 `.py` / `.ps1` 脚本
- 高风险动作需要确认（UI 风险分层：Safe / Balanced / Advanced）
- 执行结果和失败信息写入可审计日志

## 每次发布文案更新规范

每次 push / release 同步更新 README 的介绍内容，避免“代码变了，介绍没变”：

1. 更新“当前状态”日期与能力快照
2. 增加本次新增功能的用户视角描述（不是仅写技术术语）
3. 删除或降级任何尚未完成能力，避免误导
4. 增加至少 1 条可直接复制运行的新命令示例
5. 更新依赖说明（如新增脚本依赖）
6. 若能力有边界，必须同步写出限制与风险说明

建议在每次合并前执行一次：

```bash
cd apps/desktop
npm run test:smoke
```

再检查 README 是否已反映真实可运行状态。

## 贡献方向（欢迎加入）

- 桌面技能执行器扩展（真实操作、可回放日志）
- 屏幕理解和意图识别准确率提升
- 多语言命令解析（中英混合）
- 老年友好与低视力可访问性优化
- 语音交互链路（后续阶段）
- 安全策略与风险控制增强

## 项目结构

- `apps/desktop`：Tauri + React 桌面应用
- `docs/skills`：技能文档与外部参考研究
- `ARCHITECTURE.md`：系统架构
- `ROADMAP.md`：阶段路线图
- `CONTRIBUTING.md`：贡献与协作规范

## License

See repository license and third-party dependency licenses before redistribution.

