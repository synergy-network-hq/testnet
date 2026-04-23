#!/usr/bin/env python3
"""Minimal HTTP seed service for Testnet-Beta public bootstrap discovery."""

from __future__ import annotations

import argparse
import json
import os
import re
from dataclasses import dataclass, field
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


def normalize_dial(value: str) -> str | None:
    raw = (value or "").strip()
    if not raw:
        return None

    if raw.startswith(("http://", "https://")):
        parsed = urlparse(raw)
        if parsed.hostname and parsed.port:
            return f"{parsed.hostname}:{parsed.port}"
        return None

    if raw.startswith("snr://"):
        raw = raw.split("://", 1)[1]
    if "@" in raw:
        raw = raw.rsplit("@", 1)[1]
    raw = raw.split("/", 1)[0].strip()
    if not raw or ":" not in raw:
        return None

    host, port = raw.rsplit(":", 1)
    host = host.strip().strip("[]")
    if not host:
        return None
    try:
        port_num = int(port)
    except ValueError:
        return None
    if port_num <= 0 or port_num > 65535:
        return None
    return f"{host}:{port_num}"


def to_dnsaddr(dial: str) -> str | None:
    normalized = normalize_dial(dial)
    if not normalized:
        return None
    host, port = normalized.rsplit(":", 1)
    if re.fullmatch(r"\d+\.\d+\.\d+\.\d+", host):
        return f"dnsaddr=/ip4/{host}/tcp/{port}"
    return f"dnsaddr=/dns4/{host}/tcp/{port}"


@dataclass
class SeedConfig:
    label: str
    listen_host: str = "0.0.0.0"
    port: int = 5621
    admin_token_env: str = "SEED_ADMIN_TOKEN"
    allow_dynamic_registration: bool = False
    state_file: str = ""
    bootnodes: list[dict[str, Any]] = field(default_factory=list)
    static_peers: list[str] = field(default_factory=list)
    dnsaddr_bootstrap: list[str] = field(default_factory=list)


class SeedState:
    def __init__(self, config: SeedConfig) -> None:
        self.config = config
        self.dynamic_peers: dict[str, dict[str, Any]] = {}
        self.state_path = Path(config.state_file).expanduser() if config.state_file else None
        self._load()

    def _load(self) -> None:
        if not self.state_path or not self.state_path.exists():
            return
        try:
            payload = json.loads(self.state_path.read_text(encoding="utf-8"))
        except Exception:
            return
        peers = payload.get("dynamic_peers", [])
        for entry in peers:
            dial = normalize_dial(str(entry.get("dial", "")))
            if dial:
                self.dynamic_peers[dial] = {
                    "dial": dial,
                    "node_id": str(entry.get("node_id", "")).strip(),
                    "role_id": str(entry.get("role_id", "")).strip(),
                    "wallet_address": str(entry.get("wallet_address", "")).strip(),
                }

    def _save(self) -> None:
        if not self.state_path:
            return
        self.state_path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "dynamic_peers": sorted(self.dynamic_peers.values(), key=lambda item: item["dial"]),
        }
        self.state_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    def peer_list_payload(self) -> dict[str, Any]:
        peers = {normalize_dial(peer) for peer in self.config.static_peers}
        peers.update(self.dynamic_peers.keys())
        peers.discard(None)
        dns_records = list(self.config.dnsaddr_bootstrap)
        for peer in sorted(peers):
            record = to_dnsaddr(peer)
            if record and record not in dns_records:
                dns_records.append(record)
        return {
            "label": self.config.label,
            "bootnodes": self.config.bootnodes,
            "dnsaddr_bootstrap": dns_records,
            "peers": sorted(peers),
        }

    def register(self, payload: dict[str, Any]) -> tuple[bool, dict[str, Any]]:
        dial = normalize_dial(str(payload.get("dial", "")))
        if not dial:
            return False, {"ok": False, "error": "Missing or invalid dial"}
        if not self.config.allow_dynamic_registration:
            return True, {"ok": True, "accepted": False, "reason": "dynamic registration disabled"}
        self.dynamic_peers[dial] = {
            "dial": dial,
            "node_id": str(payload.get("node_id", "")).strip(),
            "role_id": str(payload.get("role_id", "")).strip(),
            "wallet_address": str(payload.get("wallet_address", "")).strip(),
        }
        self._save()
        return True, {"ok": True, "accepted": True, "dial": dial}

    def clear(self) -> None:
        self.dynamic_peers.clear()
        self._save()


class SeedHandler(BaseHTTPRequestHandler):
    server_version = "SynergySeed/1.0"

    @property
    def state(self) -> SeedState:
        return self.server.seed_state  # type: ignore[attr-defined]

    def _write_json(self, status: HTTPStatus, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, indent=2).encode("utf-8")
        self.send_response(status.value)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _write_text(self, status: HTTPStatus, body: str) -> None:
        encoded = body.encode("utf-8")
        self.send_response(status.value)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def _read_json(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0") or "0")
        raw = self.rfile.read(length) if length > 0 else b"{}"
        return json.loads(raw.decode("utf-8") or "{}")

    def _is_admin(self) -> bool:
        token_name = self.state.config.admin_token_env
        expected = os.environ.get(token_name, "").strip()
        supplied = self.headers.get("X-Seed-Admin-Token", "").strip()
        if expected and supplied == expected:
            return True
        host = self.client_address[0]
        return host in {"127.0.0.1", "::1"}

    def log_message(self, format: str, *args: Any) -> None:
        return

    def do_GET(self) -> None:  # noqa: N802
        if self.path in {"/", "/healthz"}:
            self._write_text(HTTPStatus.OK, "ok\n")
            return
        if self.path == "/peer-list.json":
            self._write_json(HTTPStatus.OK, self.state.peer_list_payload())
            return
        if self.path == "/dns/bootstrap.txt":
            payload = self.state.peer_list_payload()
            body = "\n".join(payload["dnsaddr_bootstrap"]) + "\n"
            self._write_text(HTTPStatus.OK, body)
            return
        if self.path == "/peers":
            self._write_json(
                HTTPStatus.OK,
                {"ok": True, "dynamic_peers": sorted(self.state.dynamic_peers.values(), key=lambda item: item["dial"])},
            )
            return
        self._write_json(HTTPStatus.NOT_FOUND, {"ok": False, "error": "Not found"})

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/peers/register":
            self._write_json(HTTPStatus.NOT_FOUND, {"ok": False, "error": "Not found"})
            return
        try:
            payload = self._read_json()
        except json.JSONDecodeError:
            self._write_json(HTTPStatus.BAD_REQUEST, {"ok": False, "error": "Invalid JSON"})
            return
        ok, response = self.state.register(payload)
        self._write_json(HTTPStatus.OK if ok else HTTPStatus.BAD_REQUEST, response)

    def do_DELETE(self) -> None:  # noqa: N802
        if self.path != "/peers":
            self._write_json(HTTPStatus.NOT_FOUND, {"ok": False, "error": "Not found"})
            return
        if not self._is_admin():
            self._write_json(HTTPStatus.FORBIDDEN, {"ok": False, "error": "Admin token required"})
            return
        self.state.clear()
        self._write_json(HTTPStatus.OK, {"ok": True, "cleared": True})


def load_config(path: Path) -> SeedConfig:
    payload = json.loads(path.read_text(encoding="utf-8"))
    return SeedConfig(
        label=str(payload.get("label", path.stem)).strip() or path.stem,
        listen_host=str(payload.get("listen_host", "0.0.0.0")).strip() or "0.0.0.0",
        port=int(payload.get("port", 5621)),
        admin_token_env=str(payload.get("admin_token_env", "SEED_ADMIN_TOKEN")).strip() or "SEED_ADMIN_TOKEN",
        allow_dynamic_registration=bool(payload.get("allow_dynamic_registration", False)),
        state_file=str(payload.get("state_file", "")).strip(),
        bootnodes=list(payload.get("bootnodes", [])),
        static_peers=list(payload.get("static_peers", [])),
        dnsaddr_bootstrap=list(payload.get("dnsaddr_bootstrap", [])),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Run the Synergy Testnet-Beta seed service.")
    parser.add_argument("--config", required=True, help="Path to the seed service JSON config")
    args = parser.parse_args()

    config = load_config(Path(args.config).expanduser())
    state = SeedState(config)
    server = ThreadingHTTPServer((config.listen_host, config.port), SeedHandler)
    server.daemon_threads = True
    server.seed_state = state  # type: ignore[attr-defined]
    print(f"Seed service '{config.label}' listening on {config.listen_host}:{config.port}")
    server.serve_forever()


if __name__ == "__main__":
    main()
