#!/usr/bin/env python3
"""
xixi chat platform bridge (local gateway)

Purpose:
- Accept chat callbacks (Feishu first) and enqueue remote commands for xixi.
- xixi desktop app polls the queue and executes supported commands.

Queue path:
- %LOCALAPPDATA%\\xixi\\bridge\\inbox.jsonl
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import uuid
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Dict, Optional
from urllib.parse import urlparse


def utc_now_iso() -> str:
    return dt.datetime.utcnow().isoformat(timespec="seconds") + "Z"


def log(msg: str) -> None:
    print(f"[{utc_now_iso()}] {msg}", flush=True)


def bridge_dir_path() -> Path:
    local = os.environ.get("LOCALAPPDATA", "").strip()
    if local:
        return Path(local) / "xixi" / "bridge"
    return Path.cwd() / ".xixi-bridge"


def bridge_inbox_path() -> Path:
    return bridge_dir_path() / "inbox.jsonl"


def ensure_bridge_dir() -> None:
    bridge_dir_path().mkdir(parents=True, exist_ok=True)


def extract_command_from_text(text: str) -> str:
    text = (text or "").strip()
    if not text:
        return ""

    lower = text.lower()
    for prefix in ("/xixi", "xixi"):
        if lower.startswith(prefix):
            tail = text[len(prefix) :].strip()
            while tail.startswith((":", "：", ",", "，")):
                tail = tail[1:].strip()
            return tail

    if os.environ.get("XIXI_BRIDGE_ACCEPT_PLAIN_TEXT", "0") == "1":
        return text
    return ""


def parse_feishu_text(payload: Dict[str, Any]) -> str:
    event = payload.get("event") or {}
    message = event.get("message") or {}
    if message.get("message_type") != "text":
        return ""
    content_raw = message.get("content") or ""
    if not isinstance(content_raw, str) or not content_raw.strip():
        return ""
    try:
        content_obj = json.loads(content_raw)
    except json.JSONDecodeError:
        return ""
    return (content_obj.get("text") or "").strip()


def verify_feishu_signature(headers: Dict[str, str], body_bytes: bytes) -> bool:
    encrypt_key = os.environ.get("FEISHU_ENCRYPT_KEY", "").strip()
    if not encrypt_key:
        return True

    timestamp = headers.get("X-Lark-Request-Timestamp", "")
    nonce = headers.get("X-Lark-Request-Nonce", "")
    signature = headers.get("X-Lark-Signature", "")
    if not timestamp or not nonce or not signature:
        return False

    raw = (timestamp + nonce + encrypt_key).encode("utf-8") + body_bytes
    expected = hashlib.sha256(raw).hexdigest()
    return expected == signature


def verify_feishu_token(payload: Dict[str, Any]) -> bool:
    expected = os.environ.get("FEISHU_VERIFICATION_TOKEN", "").strip()
    if not expected:
        return True

    token = payload.get("token")
    if not token:
        token = (payload.get("header") or {}).get("token")
    return token == expected


def enqueue_remote_command(source: str, text: str, payload: Dict[str, Any]) -> Dict[str, Any]:
    command_text = extract_command_from_text(text)
    if not command_text:
        return {
            "accepted": False,
            "reason": "message does not start with xixi prefix",
        }

    ensure_bridge_dir()
    now_ms = int(dt.datetime.utcnow().timestamp() * 1000)
    cmd_id = f"remote-{now_ms}-{uuid.uuid4().hex[:8]}"
    record = {
        "id": cmd_id,
        "source": source,
        "text": command_text,
        "received_at_ms": now_ms,
        "meta": {
            "raw_text": text[:500],
            "payload_type": payload.get("type") or (payload.get("header") or {}).get("event_type", ""),
        },
    }

    inbox = bridge_inbox_path()
    with inbox.open("a", encoding="utf-8") as f:
        f.write(json.dumps(record, ensure_ascii=False))
        f.write("\n")

    log(f"queued remote command id={cmd_id} source={source} text={command_text}")
    return {"accepted": True, "id": cmd_id, "text": command_text}


class BridgeHandler(BaseHTTPRequestHandler):
    server_version = "xixi-bridge/0.1"

    def _json_response(self, status: int, obj: Dict[str, Any]) -> None:
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json_body(self) -> tuple[Optional[Dict[str, Any]], bytes]:
        length = int(self.headers.get("Content-Length", "0") or "0")
        body = self.rfile.read(length) if length > 0 else b""
        if not body:
            return None, body
        try:
            return json.loads(body.decode("utf-8")), body
        except json.JSONDecodeError:
            return None, body

    def _check_ingest_token(self) -> bool:
        expected = os.environ.get("XIXI_BRIDGE_TOKEN", "").strip()
        if not expected:
            return True
        auth = self.headers.get("Authorization", "").strip()
        if auth.lower().startswith("bearer "):
            token = auth[7:].strip()
            return token == expected
        return False

    def do_GET(self) -> None:  # noqa: N802
        route = urlparse(self.path).path
        if route == "/health":
            self._json_response(
                HTTPStatus.OK,
                {
                    "ok": True,
                    "service": "xixi-chat-bridge",
                    "inbox": str(bridge_inbox_path()),
                },
            )
            return
        self._json_response(HTTPStatus.NOT_FOUND, {"ok": False, "error": "not found"})

    def do_POST(self) -> None:  # noqa: N802
        route = urlparse(self.path).path
        payload, body = self._read_json_body()

        if route == "/feishu/events":
            if payload is None:
                self._json_response(HTTPStatus.BAD_REQUEST, {"ok": False, "error": "invalid json"})
                return
            if not verify_feishu_signature(dict(self.headers), body):
                self._json_response(HTTPStatus.FORBIDDEN, {"ok": False, "error": "bad feishu signature"})
                return
            if not verify_feishu_token(payload):
                self._json_response(HTTPStatus.FORBIDDEN, {"ok": False, "error": "bad feishu token"})
                return

            if payload.get("type") == "url_verification":
                self._json_response(HTTPStatus.OK, {"challenge": payload.get("challenge", "")})
                return

            text = parse_feishu_text(payload)
            source = "feishu"
            chat_id = ((payload.get("event") or {}).get("message") or {}).get("chat_id")
            if isinstance(chat_id, str) and chat_id:
                source = f"feishu:{chat_id}"
            result = enqueue_remote_command(source, text, payload)
            self._json_response(HTTPStatus.OK, {"ok": True, "result": result})
            return

        if route == "/ingest":
            if not self._check_ingest_token():
                self._json_response(HTTPStatus.FORBIDDEN, {"ok": False, "error": "invalid bridge token"})
                return
            if payload is None:
                self._json_response(HTTPStatus.BAD_REQUEST, {"ok": False, "error": "invalid json"})
                return
            text = str(payload.get("text") or "")
            source = str(payload.get("source") or "external")
            result = enqueue_remote_command(source, text, payload)
            self._json_response(HTTPStatus.OK, {"ok": True, "result": result})
            return

        self._json_response(HTTPStatus.NOT_FOUND, {"ok": False, "error": "not found"})

    def log_message(self, fmt: str, *args: Any) -> None:
        log(f"http {self.address_string()} {fmt % args}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="xixi chat bridge local gateway")
    parser.add_argument("--host", default="0.0.0.0", help="bind host, default 0.0.0.0")
    parser.add_argument("--port", type=int, default=17770, help="bind port, default 17770")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    ensure_bridge_dir()
    server = ThreadingHTTPServer((args.host, args.port), BridgeHandler)
    log(f"xixi chat bridge started at http://{args.host}:{args.port}")
    log(f"inbox path: {bridge_inbox_path()}")
    log("routes: GET /health, POST /feishu/events, POST /ingest")
    server.serve_forever()


if __name__ == "__main__":
    main()
