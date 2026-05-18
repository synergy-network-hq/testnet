#!/usr/bin/env python3
"""Portable Synergy Testnet transaction helper with embedded testnet keys.

This file intentionally embeds the shared Faucet and Token Sales TESTNET keys.
Do not adapt this pattern for mainnet or any wallet with real value.
"""
from __future__ import annotations

import argparse
import base64
import json
import os
import platform
import math
import shutil
import stat
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path

DEFAULT_RPC_URL = "https://testnet-core-rpc.synergy-network.io"
NWEI_PER_SNRG = Decimal("1000000000")
DEFAULT_GAS_PRICE = 1000
DEFAULT_GAS_LIMIT = 21000
EMBEDDED_WALLETS = {
    "faucet": {
        "address": "synw1zp7cxme7xm838663yrd43lxtxlw0ck90z4am",
        "private_key": "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"
    },
    "token-sales": {
        "address": "synw17nh265ug2fgc8guv2ad7tt8kv0wlhesxndl8",
        "private_key": "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"
    }
}


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def rpc_call(rpc_url: str, method: str, params: list, timeout: int = 25) -> dict:
    payload = json.dumps({"jsonrpc":"2.0","method":method,"params":params,"id":1}, separators=(",", ":")).encode()
    request = urllib.request.Request(rpc_url, data=payload, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        result = json.loads(response.read().decode())
    if result.get("error"):
        raise RuntimeError(result["error"].get("message") if isinstance(result["error"], dict) else str(result["error"]))
    return result


def wallet_label_or_address(value: str) -> str:
    return EMBEDDED_WALLETS.get(value, {"address": value})["address"]


def load_wallet(label: str) -> dict:
    if label not in EMBEDDED_WALLETS:
        raise ValueError(f"unknown embedded wallet {label!r}; use one of: {', '.join(sorted(EMBEDDED_WALLETS))}")
    wallet = dict(EMBEDDED_WALLETS[label])
    wallet["label"] = label
    wallet["private_key_hex"] = base64.b64decode(wallet["private_key"]).hex()
    return wallet


def format_snrg(nwei: int) -> str:
    value = Decimal(nwei) / NWEI_PER_SNRG
    text = f"{value:.9f}"
    return text.rstrip("0").rstrip(".") if "." in text else text


def amount_to_nwei(amount_snrg: str | None, amount_nwei: int | None) -> int:
    if amount_nwei is not None:
        if amount_nwei <= 0:
            raise ValueError("amount nWei must be positive")
        return amount_nwei
    if amount_snrg is None:
        amount_snrg = "1"
    try:
        amount = Decimal(amount_snrg)
    except InvalidOperation as exc:
        raise ValueError("amount SNRG must be a valid decimal") from exc
    if amount <= 0:
        raise ValueError("amount SNRG must be positive")
    nwei = amount * NWEI_PER_SNRG
    if nwei != nwei.to_integral_value():
        raise ValueError("amount SNRG supports no more than 9 decimal places")
    return int(nwei)


def resolve_wallet_cli(cli_arg: str | None = None) -> str:
    candidates = []
    if cli_arg:
        candidates.append(Path(cli_arg))
    if os.environ.get("SYNERGY_WALLET_CLI"):
        candidates.append(Path(os.environ["SYNERGY_WALLET_CLI"]))

    here = Path(__file__).resolve().parent
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin" and machine in ("arm64", "aarch64"):
        candidates.append(here / "wallet-pqc-cli-darwin-arm64")
        candidates.append(here / "wallet-pqc-cli-macos-universal")
    elif system == "darwin":
        candidates.append(here / "wallet-pqc-cli-darwin-x64")
        candidates.append(here / "wallet-pqc-cli-macos-universal")
    elif system == "linux" and machine in ("arm64", "aarch64"):
        candidates.append(here / "wallet-pqc-cli-linux-arm64")
    elif system == "linux":
        candidates.append(here / "wallet-pqc-cli-linux-x64")
    elif system == "windows":
        candidates.append(here / "wallet-pqc-cli-windows-x64.exe")

    candidates.append(here / "wallet-pqc-cli")
    found = shutil.which("wallet-pqc-cli")
    if found:
        candidates.append(Path(found))

    for candidate in candidates:
        if candidate.is_file():
            if system != "windows":
                candidate.chmod(candidate.stat().st_mode | stat.S_IXUSR)
            return str(candidate)
    searched = "\\n  ".join(str(c) for c in candidates)
    raise FileNotFoundError(f"wallet-pqc-cli was not found. Searched:\\n  {searched}")


def build_unsigned_tx(sender: dict, receiver: str, amount_nwei: int, nonce: int, gas_price: int, gas_limit: int, algo: str, data: str | None = None) -> dict:
    return {
        "sender": sender["address"],
        "receiver": receiver,
        "amount": amount_nwei,
        "nonce": nonce,
        "signature": [],
        "timestamp": int(time.time()),
        "gas_price": gas_price,
        "gas_limit": gas_limit,
        "data": data,
        "signature_algorithm": algo,
    }


def sign_tx(wallet_cli: str, sender: dict, tx: dict, algo: str) -> dict:
    proc = subprocess.run(
        [wallet_cli, "sign-tx", "--private-key", sender["private_key_hex"], "--tx", json.dumps(tx, separators=(",", ":")), "--algo", algo],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or proc.stdout.strip() or "wallet-pqc-cli sign-tx failed")
    return json.loads(proc.stdout)["transaction"]


def submit_tx(rpc_url: str, signed_tx: dict) -> tuple[str, dict]:
    response = rpc_call(rpc_url, "synergy_sendTransaction", [signed_tx])
    result = response.get("result")
    if isinstance(result, dict) and result.get("success") is False:
        raise RuntimeError(str(result.get("error") or result))
    if isinstance(result, str):
        return result, response
    if isinstance(result, dict):
        return str(result.get("tx_hash") or result.get("hash") or ""), response
    return "", response


def wait_for_receipt(rpc_url: str, tx_hash: str, timeout_seconds: int = 60) -> bool:
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        for method in ("synergy_getTransactionReceipt", "synergy_getTransactionByHash"):
            try:
                if rpc_call(rpc_url, method, [tx_hash], timeout=10).get("result"):
                    return True
            except Exception:
                pass
        time.sleep(2)
    return False


def confirm_or_exit(args: argparse.Namespace, message: str) -> None:
    if getattr(args, "yes", False):
        return
    print(message)
    answer = input("Type yes to continue: ").strip()
    if answer != "yes":
        raise SystemExit("Cancelled. No transaction was submitted.")


def command_list_wallets(_args: argparse.Namespace) -> int:
    for label, wallet in EMBEDDED_WALLETS.items():
        print(f"{label} {wallet['address']}")
    return 0


def command_height(args: argparse.Namespace) -> int:
    print(rpc_call(args.rpc_url, "synergy_blockNumber", [])["result"])
    return 0


def command_status(args: argparse.Namespace) -> int:
    print(json.dumps(rpc_call(args.rpc_url, "synergy_getNodeStatus", [])["result"], indent=2, sort_keys=True))
    return 0


def command_balance(args: argparse.Namespace) -> int:
    address = wallet_label_or_address(args.wallet_or_address)
    result = int(rpc_call(args.rpc_url, "synergy_getTokenBalance", [address, "SNRG"])["result"] or 0)
    print(f"{address} {format_snrg(result)} SNRG ({result} nWei)")
    return 0


def command_nonce(args: argparse.Namespace) -> int:
    address = wallet_label_or_address(args.wallet_or_address)
    print(rpc_call(args.rpc_url, "synergy_getAccountNonce", [address])["result"])
    return 0


def send_once(args: argparse.Namespace, sender_label: str, receiver: str, amount_nwei: int) -> str:
    sender = load_wallet(sender_label)
    wallet_cli = resolve_wallet_cli(args.wallet_cli)
    nonce = int(rpc_call(args.rpc_url, "synergy_getAccountNonce", [sender["address"]])["result"] or 0)
    tx = build_unsigned_tx(sender, receiver, amount_nwei, nonce, args.gas_price, args.gas_limit, args.algo)
    signed = sign_tx(wallet_cli, sender, tx, args.algo)
    tx_hash, _response = submit_tx(args.rpc_url, signed)
    print(f"[{utc_now()}] OK {sender_label} -> {receiver} nonce={nonce} tx={tx_hash}")
    if args.wait:
        print(f"[{utc_now()}] receipt tx={tx_hash} confirmed={str(wait_for_receipt(args.rpc_url, tx_hash, args.receipt_timeout_seconds)).lower()}")
    return tx_hash


def command_send(args: argparse.Namespace) -> int:
    receiver = wallet_label_or_address(args.to)
    amount_nwei = amount_to_nwei(args.amount_snrg, args.amount_nwei)
    confirm_or_exit(args, f"Send {format_snrg(amount_nwei)} SNRG from {args.from_wallet} to {receiver} on Synergy Testnet.")
    send_once(args, args.from_wallet, receiver, amount_nwei)
    return 0


def command_pingpong(args: argparse.Namespace) -> int:
    amount_nwei = amount_to_nwei(args.amount_snrg, args.amount_nwei)
    total = max(1, math.ceil(args.duration_seconds / args.interval_seconds))
    if args.max_transactions is not None:
        total = min(total, args.max_transactions)
    confirm_or_exit(args, f"Run {total} alternating transfers of {format_snrg(amount_nwei)} SNRG every {args.interval_seconds} seconds.")
    start = time.monotonic()
    labels = ["faucet", "token-sales"]
    for index in range(total):
        target = start + (index * args.interval_seconds)
        if target > time.monotonic():
            time.sleep(target - time.monotonic())
        sender_label = labels[index % 2]
        receiver_label = labels[(index + 1) % 2]
        send_once(args, sender_label, EMBEDDED_WALLETS[receiver_label]["address"], amount_nwei)
    return 0


def command_burst(args: argparse.Namespace) -> int:
    amount_nwei = amount_to_nwei(args.amount_snrg, args.amount_nwei)
    senders = args.senders or ["faucet", "token-sales"]
    total = len(senders) * args.tx_per_sender
    confirm_or_exit(args, f"Run {total} signed burst transactions of {format_snrg(amount_nwei)} SNRG.")
    for seq in range(args.tx_per_sender):
        for idx, sender_label in enumerate(senders):
            if args.receiver:
                receiver = wallet_label_or_address(args.receiver)
            else:
                receiver = EMBEDDED_WALLETS[senders[(idx + 1) % len(senders)]]["address"]
            send_once(args, sender_label, receiver, amount_nwei)
            if args.interval_seconds > 0:
                time.sleep(args.interval_seconds)
    return 0


def add_common_tx_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--rpc-url", default=os.environ.get("SYNERGY_RPC_URL", DEFAULT_RPC_URL))
    parser.add_argument("--wallet-cli", default=None)
    parser.add_argument("--algo", choices=["fndsa", "mldsa", "slhdsa"], default="fndsa")
    parser.add_argument("--gas-price", type=int, default=DEFAULT_GAS_PRICE)
    parser.add_argument("--gas-limit", type=int, default=DEFAULT_GAS_LIMIT)
    parser.add_argument("--wait", action="store_true")
    parser.add_argument("--receipt-timeout-seconds", type=int, default=60)
    parser.add_argument("--yes", action="store_true")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Portable Synergy Testnet helper with embedded testnet signing keys.")
    parser.add_argument("--rpc-url", default=os.environ.get("SYNERGY_RPC_URL", DEFAULT_RPC_URL), help=argparse.SUPPRESS)
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("list-wallets", help="List embedded testnet wallet aliases and addresses")
    p.set_defaults(func=command_list_wallets)

    p = sub.add_parser("height", help="Print current block height")
    p.add_argument("--rpc-url", default=os.environ.get("SYNERGY_RPC_URL", DEFAULT_RPC_URL))
    p.set_defaults(func=command_height)

    p = sub.add_parser("status", help="Print node status JSON")
    p.add_argument("--rpc-url", default=os.environ.get("SYNERGY_RPC_URL", DEFAULT_RPC_URL))
    p.set_defaults(func=command_status)

    p = sub.add_parser("balance", help="Print SNRG balance for faucet, token-sales, or an address")
    p.add_argument("wallet_or_address")
    p.add_argument("--rpc-url", default=os.environ.get("SYNERGY_RPC_URL", DEFAULT_RPC_URL))
    p.set_defaults(func=command_balance)

    p = sub.add_parser("nonce", help="Print account nonce for faucet, token-sales, or an address")
    p.add_argument("wallet_or_address")
    p.add_argument("--rpc-url", default=os.environ.get("SYNERGY_RPC_URL", DEFAULT_RPC_URL))
    p.set_defaults(func=command_nonce)

    p = sub.add_parser("send", help="Sign and submit one native SNRG transfer")
    p.add_argument("--from", dest="from_wallet", choices=sorted(EMBEDDED_WALLETS), required=True)
    p.add_argument("--to", required=True, help="Recipient address, faucet, or token-sales")
    p.add_argument("--amount-snrg", default="1")
    p.add_argument("--amount-nwei", type=int, default=None)
    add_common_tx_args(p)
    p.set_defaults(func=command_send)

    p = sub.add_parser("pingpong", help="Alternate faucet <-> token-sales transfers")
    p.add_argument("--duration-seconds", type=float, default=3600.0)
    p.add_argument("--interval-seconds", type=float, default=5.0)
    p.add_argument("--amount-snrg", default="1")
    p.add_argument("--amount-nwei", type=int, default=None)
    p.add_argument("--max-transactions", type=int, default=None)
    add_common_tx_args(p)
    p.set_defaults(func=command_pingpong)

    p = sub.add_parser("burst", help="Send repeated signed transactions from embedded wallets")
    p.add_argument("--senders", nargs="+", choices=sorted(EMBEDDED_WALLETS), default=None)
    p.add_argument("--receiver", default=None, help="Receiver address/alias. Defaults to ring routing among senders.")
    p.add_argument("--tx-per-sender", type=int, default=3)
    p.add_argument("--interval-seconds", type=float, default=1.0)
    p.add_argument("--amount-snrg", default=None)
    p.add_argument("--amount-nwei", type=int, default=1)
    add_common_tx_args(p)
    p.set_defaults(func=command_burst)

    return parser.parse_args()


def main() -> int:
    args = parse_args()
    return args.func(args)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print(f"\\n[{utc_now()}] interrupted", file=sys.stderr)
        raise SystemExit(130)
