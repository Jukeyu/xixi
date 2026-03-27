# xixi Chat Platform Bridge (Remote Control)

This document tracks real progress for using chat platforms to remotely trigger xixi commands on a home computer.

## Goal

When xixi is running at home, a user can send a message from phone chat, and xixi executes supported desktop actions.

Current implementation path:

1. Chat platform callback -> local bridge service.
2. Bridge service writes commands into `%LOCALAPPDATA%\xixi\bridge\inbox.jsonl`.
3. xixi desktop app polls inbox and executes commands through existing planner/executor.

## Current Status (2026-03-27)

- Feishu callback ingestion: implemented (`scripts/xixi_chat_bridge.py`).
- Generic external ingestion endpoint: implemented (`POST /ingest`).
- xixi inbox polling and execution loop: implemented in desktop UI settings.
- WeCom / WeChat dedicated callback adapters: not implemented yet (research complete, architecture reserved).

## What We Built

## 1. Local bridge service

File: `scripts/xixi_chat_bridge.py`

- `GET /health`: check service status.
- `POST /feishu/events`: handles Feishu event callbacks.
- `POST /ingest`: generic authenticated ingestion endpoint for future WeCom/WeChat relays.

Security options:

- `FEISHU_ENCRYPT_KEY`: enables Feishu signature verification for event callbacks.
- `FEISHU_VERIFICATION_TOKEN`: enables token matching.
- `XIXI_BRIDGE_TOKEN`: protects `/ingest` with `Authorization: Bearer <token>`.

Message format rule:

- By default, only messages starting with `xixi` or `/xixi` are treated as commands.
- Set `XIXI_BRIDGE_ACCEPT_PLAIN_TEXT=1` to accept plain text directly.

## 2. Desktop bridge polling

Files:

- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src/App.tsx`

New backend commands:

- `get_bridge_folder_path`
- `bridge_pull_remote_commands(limit?: number)`

New UI settings:

- `Enable remote chat bridge polling`
- `Remote poll interval (seconds)`
- `Remote bridge folder` (read-only path)

When polling is enabled in command mode, xixi:

1. Pulls one command from bridge inbox.
2. Adds a chat trace message.
3. Runs existing `plan_user_request` + execution pipeline.

## Runbook

## 1. Start xixi desktop app

```bash
cd apps/desktop
npm run tauri:dev
```

In xixi settings:

- Keep `Chat mode = command`.
- Turn on `Enable remote chat bridge polling`.
- Configure `Permission profile` according to risk policy.

## 2. Start bridge service

```bash
python scripts/xixi_chat_bridge.py --host 0.0.0.0 --port 17770
```

## 3. Feishu callback wiring

- Configure app event callback URL to:
  - `http(s)://<your-public-endpoint>/feishu/events`
- Send a text message with prefix, for example:
  - `/xixi open app calculator`
  - `/xixi open site github.com`

## 4. Generic relay test

```bash
curl -X POST http://127.0.0.1:17770/ingest \
  -H "Content-Type: application/json" \
  -d "{\"source\":\"manual\",\"text\":\"/xixi open app notepad\"}"
```

## Platform Research Notes (Official Sources)

## Feishu

- Custom bot webhook format and safety notes (keep webhook secret private):
  - https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot
- Custom bot rate limit and payload size notes:
  - https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot
- App bot send message API:
  - https://open.feishu.cn/document/server-docs/im-v1/message/create
- Event subscription overview and long connection mode:
  - https://open.feishu.cn/document/server-docs/event-subscription-guide/overview
- Event callback security (signature / token) and sample algorithm:
  - https://open.feishu.cn/document/server-docs/event-subscription-guide/event-subscription-configure-/encrypt-key-encryption-configuration-case

## WeCom (Enterprise WeChat)

- Group robot webhook send URL pattern and webhook secrecy warning:
  - https://developer.work.weixin.qq.com/document/path/91770
- App message send API (`message/send` with `access_token`):
  - https://developer.work.weixin.qq.com/document/path/90236
- Access token API (`gettoken`, `expires_in`):
  - https://developer.work.weixin.qq.com/document/path/91039

## WeChat Open Platform Scope

- Official open platform scenarios emphasize mobile app, website login, public-account / mini-program ecosystem:
  - https://open.weixin.qq.com

## Practical Direction Decision

- Near-term production path: Feishu + WeCom first (official enterprise APIs are clearer and safer).
- WeChat personal-account automation: do not rely on unofficial reverse-engineering path for production.
- Add WeCom and WeChat adapters through `/ingest` relay while preserving xixi local execution and audit logs.

## Safety and Ops Checklist

- Never expose webhook URLs or tokens in public repos.
- Keep permission profile in `safe`/`balanced` for unattended mode.
- Keep high-risk actions confirmation policy enabled.
- Audit `%LOCALAPPDATA%\xixi\skills\runs` and bridge inbox regularly.
