#!/usr/bin/env python3
import hmac
import ipaddress
import json
import os
import socket
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse


BASE_DIR = Path(__file__).resolve().parent
CONFIG_PATH = BASE_DIR / "config" / "seed-service.json"
PEER_PATH = BASE_DIR / "data" / "peers.json"
PEER_LOCK = threading.Lock()
STATE = {"generated_at": None, "bootnodes": [], "peers": [], "peer_updated_at": None}

PEER_TTL_SECONDS = 300


def load_config():
    return json.loads(CONFIG_PATH.read_text())


def load_peers():
    if not PEER_PATH.exists():
        return []
    try:
        raw = json.loads(PEER_PATH.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    if isinstance(raw, dict):
        raw = raw.get("peers", [])
    if not isinstance(raw, list):
        return []
    return dedupe_peer_records(raw)


def save_peers(peers):
    PEER_PATH.parent.mkdir(parents=True, exist_ok=True)
    PEER_PATH.write_text(json.dumps(dedupe_peer_records(peers), indent=2))


def parse_int(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def is_plausible_dial_host(host):
    normalized = str(host or "").strip()
    if not normalized:
        return False
    if normalized.lower() == "localhost":
        return True
    try:
        ipaddress.ip_address(normalized)
        return True
    except ValueError:
        pass
    return "." in normalized and all(
        character.isalnum() or character in "-." for character in normalized
    )


def normalize_host_port(host, port):
    normalized_host = str(host or "").strip().strip("[]").rstrip(".")
    normalized_port = parse_int(port)
    if (
        normalized_port is None
        or normalized_port <= 0
        or normalized_port > 65535
        or not normalized_host
        or not is_plausible_dial_host(normalized_host)
    ):
        return None
    try:
        ip_value = ipaddress.ip_address(normalized_host)
    except ValueError:
        if ":" in normalized_host:
            return f"[{normalized_host}]:{normalized_port}"
        return f"{normalized_host}:{normalized_port}"

    if isinstance(ip_value, ipaddress.IPv6Address):
        return f"[{normalized_host}]:{normalized_port}"
    return f"{normalized_host}:{normalized_port}"


def normalize_dial_target(value):
    raw = str(value or "").strip()
    if not raw:
        return None

    if "://" in raw:
        raw = raw.split("://", 1)[1]
    if "@" in raw:
        raw = raw.rsplit("@", 1)[1]
    raw = raw.split("/", 1)[0]
    raw = raw.split("?", 1)[0]
    raw = raw.split("#", 1)[0]
    raw = raw.strip()
    if not raw:
        return None

    if raw.startswith("[") and "]:" in raw:
        host, port = raw[1:].rsplit("]:", 1)
        return normalize_host_port(host, port)
    if ":" not in raw:
        return None
    host, port = raw.rsplit(":", 1)
    return normalize_host_port(host, port)


def normalize_peer_payload(payload):
    if not isinstance(payload, dict):
        return None
    dial = payload.get("dial") or payload.get("peer") or payload.get("address")
    host = payload.get("public_host") or payload.get("host") or payload.get("hostname")
    port = payload.get("p2p_port") or payload.get("port")
    if not dial and host and port:
        dial = f"{host}:{port}"
    dial = normalize_dial_target(dial)
    if not dial:
        return None
    dial_host = dial[1:].split("]:", 1)[0] if dial.startswith("[") else dial.rsplit(":", 1)[0]
    dial_port = parse_int(dial.rsplit(":", 1)[1])
    updated_at = parse_int(payload.get("updated_at")) or int(time.time())
    return {
        "node_id": payload.get("node_id"),
        "role_id": payload.get("role_id"),
        "wallet_address": payload.get("wallet_address"),
        "public_host": str(host or dial_host).strip() or dial_host,
        "p2p_port": dial_port,
        "dial": dial,
        "updated_at": updated_at,
    }


def is_peer_expired(peer, now=None):
    if now is None:
        now = int(time.time())
    updated_at = parse_int(peer.get("updated_at"))
    if updated_at is None:
        return True
    return (now - updated_at) > PEER_TTL_SECONDS


def evict_expired_peers(peers):
    now = int(time.time())
    return [peer for peer in peers if not is_peer_expired(peer, now)]


def merge_peer(peers, incoming):
    dial = incoming.get("dial")
    node_id = incoming.get("node_id")

    for idx, peer in enumerate(peers):
        if dial and peer.get("dial") == dial:
            merged = dict(peer)
            merged.update(incoming)
            peers[idx] = merged
            return peers

    if node_id:
        for idx, peer in enumerate(peers):
            if peer.get("node_id") == node_id:
                merged = dict(peer)
                merged.update(incoming)
                peers[idx] = merged
                return peers

    peers.append(incoming)
    return peers


def dedupe_peer_records(records):
    seen_dials = {}
    peers = []
    for entry in records:
        normalized = normalize_peer_payload(entry)
        if not normalized:
            continue
        dial = normalized.get("dial")
        if not dial:
            continue
        if dial in seen_dials:
            existing_idx = seen_dials[dial]
            existing_updated = parse_int(peers[existing_idx].get("updated_at")) or 0
            incoming_updated = parse_int(normalized.get("updated_at")) or 0
            if incoming_updated >= existing_updated:
                peers[existing_idx] = normalized
            continue
        seen_dials[dial] = len(peers)
        peers.append(normalized)
    return peers


def peer_dials(peers):
    dials = {
        entry.get("dial")
        for entry in dedupe_peer_records(peers)
        if entry.get("dial")
    }
    return sorted(dials)


def expected_admin_token(config):
    token = str(os.environ.get("SEED_ADMIN_TOKEN") or config.get("admin_token") or "").strip()
    return token or None


def request_is_loopback(handler):
    try:
        return ipaddress.ip_address(handler.client_address[0]).is_loopback
    except ValueError:
        return handler.client_address[0] in {"localhost"}


def request_admin_token(handler):
    bearer = str(handler.headers.get("Authorization") or "").strip()
    if bearer.lower().startswith("bearer "):
        return bearer[7:].strip()
    return str(handler.headers.get("X-Seed-Admin-Token") or "").strip()


def is_admin_authorized(handler, config):
    if request_is_loopback(handler):
        return True
    expected = expected_admin_token(config)
    provided = request_admin_token(handler)
    if not expected or not provided:
        return False
    return hmac.compare_digest(provided, expected)


def check_bootnode(host, port, timeout=1.5):
    started = time.time()
    try:
        with socket.create_connection((host, port), timeout=timeout):
            latency_ms = int((time.time() - started) * 1000)
            return {"reachable": True, "latency_ms": latency_ms}
    except OSError as exc:
        return {"reachable": False, "error": str(exc)}


def rebuild_state(config):
    snapshot = []
    for entry in config["bootnodes"]:
        status = check_bootnode(entry["hostname"], entry["port"])
        merged = dict(entry)
        merged.update(status)
        snapshot.append(merged)

    STATE["generated_at"] = int(time.time())
    STATE["bootnodes"] = snapshot
    with PEER_LOCK:
        peers = evict_expired_peers(load_peers())
        save_peers(peers)
        STATE["peers"] = peers
        STATE["peer_updated_at"] = int(time.time())


def refresh_loop(config):
    interval = max(int(config.get("refresh_seconds", 30)), 5)
    while True:
        rebuild_state(config)
        time.sleep(interval)


class Handler(BaseHTTPRequestHandler):
    def _send_json(self, payload, status=200):
        body = json.dumps(payload, indent=2).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _clear_registered_peers(self, config):
        if not is_admin_authorized(self, config):
            self._send_json({"error": "forbidden"}, status=403)
            return

        with PEER_LOCK:
            cleared = len(load_peers())
            peers = []
            save_peers(peers)
            STATE["peers"] = peers
            STATE["peer_updated_at"] = int(time.time())

        self._send_json(
            {
                "ok": True,
                "cleared": cleared,
                "remaining": 0,
            }
        )

    def _deregister_peer(self, config, payload):
        node_id = payload.get("node_id")
        dial = payload.get("dial")
        if not node_id and not dial:
            self._send_json({"error": "node_id or dial required"}, status=400)
            return

        with PEER_LOCK:
            peers = load_peers()
            before_count = len(peers)
            remaining = []
            for peer in peers:
                if node_id and peer.get("node_id") == node_id:
                    continue
                if dial and peer.get("dial") == dial:
                    continue
                remaining.append(peer)
            save_peers(remaining)
            STATE["peers"] = dedupe_peer_records(remaining)
            STATE["peer_updated_at"] = int(time.time())

        removed = before_count - len(remaining)
        self._send_json(
            {
                "ok": True,
                "removed": removed,
                "remaining": len(remaining),
            }
        )

    def log_message(self, fmt, *args):
        return

    def do_GET(self):
        config = load_config()
        path = urlparse(self.path).path

        if path == "/" or path == "/healthz":
            self._send_json(
                {
                    "ok": True,
                    "service": config["service_name"],
                    "generated_at": STATE["generated_at"],
                }
            )
            return

        if path == "/peer-list.json":
            live_peers = evict_expired_peers(STATE["peers"])
            self._send_json(
                {
                    "service": config["service_name"],
                    "public_url": config["public_url"],
                    "generated_at": STATE["generated_at"],
                    "bootnodes": STATE["bootnodes"],
                    "seed_services": config["seed_services"],
                    "peers": peer_dials(live_peers),
                    "dnsaddr_bootstrap": [
                        f"dnsaddr=/dns/{entry['hostname']}/tcp/{entry['port']}"
                        for entry in config["bootnodes"]
                    ],
                }
            )
            return

        if path == "/dns/bootstrap.txt":
            lines = [
                f"dnsaddr=/dns/{entry['hostname']}/tcp/{entry['port']}"
                for entry in config["bootnodes"]
            ]
            body = ("\n".join(lines) + "\n").encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if path == "/peers":
            live_peers = evict_expired_peers(STATE["peers"])
            self._send_json(
                {
                    "service": config["service_name"],
                    "generated_at": STATE["generated_at"],
                    "peer_updated_at": STATE["peer_updated_at"],
                    "peer_ttl_seconds": PEER_TTL_SECONDS,
                    "peers": dedupe_peer_records(live_peers),
                }
            )
            return

        self._send_json({"error": "not_found"}, status=404)

    def do_POST(self):
        config = load_config()
        path = urlparse(self.path).path
        if path in {"/peers/clear", "/admin/peers/clear"}:
            self._clear_registered_peers(config)
            return

        if path == "/peers/deregister":
            length = int(self.headers.get("Content-Length", 0) or 0)
            if length <= 0:
                self._send_json({"error": "missing_body"}, status=400)
                return
            try:
                payload = json.loads(self.rfile.read(length))
            except json.JSONDecodeError:
                self._send_json({"error": "invalid_json"}, status=400)
                return
            self._deregister_peer(config, payload)
            return

        if path != "/peers/register":
            self._send_json({"error": "not_found"}, status=404)
            return

        length = int(self.headers.get("Content-Length", 0) or 0)
        if length <= 0:
            self._send_json({"error": "missing_body"}, status=400)
            return

        try:
            payload = json.loads(self.rfile.read(length))
        except json.JSONDecodeError:
            self._send_json({"error": "invalid_json"}, status=400)
            return

        registrations = payload if isinstance(payload, list) else [payload]
        updated = 0
        with PEER_LOCK:
            peers = evict_expired_peers(load_peers())
            for entry in registrations:
                normalized = normalize_peer_payload(entry)
                if not normalized:
                    continue
                normalized["updated_at"] = int(time.time())
                peers = merge_peer(peers, normalized)
                updated += 1
            save_peers(peers)
            STATE["peers"] = dedupe_peer_records(peers)
            STATE["peer_updated_at"] = int(time.time())

        if updated == 0:
            self._send_json({"error": "invalid_payload"}, status=400)
            return

        self._send_json(
            {
                "ok": True,
                "registered": updated,
                "peers": dedupe_peer_records(STATE["peers"]),
            }
        )

    def do_DELETE(self):
        config = load_config()
        path = urlparse(self.path).path
        if path in {"/peers", "/peers/clear", "/admin/peers/clear"}:
            self._clear_registered_peers(config)
            return
        self._send_json({"error": "not_found"}, status=404)


def main():
    config = load_config()
    rebuild_state(config)
    thread = threading.Thread(target=refresh_loop, args=(config,), daemon=True)
    thread.start()

    server = ThreadingHTTPServer((config["listen_host"], config["listen_port"]), Handler)
    print(
        f"Seed service {config['service_name']} listening on "
        f"{config['listen_host']}:{config['listen_port']}"
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
