#!/usr/bin/env python3
import json
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
    return [entry for entry in raw if isinstance(entry, dict)]


def save_peers(peers):
    PEER_PATH.parent.mkdir(parents=True, exist_ok=True)
    PEER_PATH.write_text(json.dumps(peers, indent=2))


def normalize_peer_payload(payload):
    if not isinstance(payload, dict):
        return None
    dial = payload.get("dial") or payload.get("peer") or payload.get("address")
    host = payload.get("public_host") or payload.get("host") or payload.get("hostname")
    port = payload.get("p2p_port") or payload.get("port")
    if not dial and host and port:
        dial = f"{host}:{port}"
    if dial:
        dial = str(dial).strip()
        if "://" not in dial:
            dial = f"snr://peer@{dial}"
    if not dial:
        return None
    try:
        port_val = int(port) if port is not None else None
    except (TypeError, ValueError):
        port_val = None
    return {
        "node_id": payload.get("node_id"),
        "role_id": payload.get("role_id"),
        "wallet_address": payload.get("wallet_address"),
        "public_host": host,
        "p2p_port": port_val,
        "dial": dial,
        "updated_at": int(time.time()),
    }


def merge_peer(peers, incoming):
    key = incoming.get("node_id") or incoming.get("dial")
    for idx, peer in enumerate(peers):
        peer_key = peer.get("node_id") or peer.get("dial")
        if peer_key == key:
            merged = dict(peer)
            merged.update(incoming)
            peers[idx] = merged
            return peers
    peers.append(incoming)
    return peers


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
        STATE["peers"] = load_peers()
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
            self._send_json(
                {
                    "service": config["service_name"],
                    "public_url": config["public_url"],
                    "generated_at": STATE["generated_at"],
                    "bootnodes": STATE["bootnodes"],
                    "seed_services": config["seed_services"],
                    "peers": [entry.get("dial") for entry in STATE["peers"] if entry.get("dial")],
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
            self._send_json(
                {
                    "service": config["service_name"],
                    "generated_at": STATE["generated_at"],
                    "peer_updated_at": STATE["peer_updated_at"],
                    "peers": STATE["peers"],
                }
            )
            return

        self._send_json({"error": "not_found"}, status=404)

    def do_POST(self):
        path = urlparse(self.path).path
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
            peers = load_peers()
            for entry in registrations:
                normalized = normalize_peer_payload(entry)
                if not normalized:
                    continue
                peers = merge_peer(peers, normalized)
                updated += 1
            save_peers(peers)
            STATE["peers"] = peers
            STATE["peer_updated_at"] = int(time.time())

        if updated == 0:
            self._send_json({"error": "invalid_payload"}, status=400)
            return

        self._send_json(
            {
                "ok": True,
                "registered": updated,
                "peers": STATE["peers"],
            }
        )


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
