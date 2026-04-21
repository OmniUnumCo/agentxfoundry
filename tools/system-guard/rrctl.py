#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import pathlib
import sqlite3
import subprocess
import sys


def run_scan(home: pathlib.Path, policy: pathlib.Path) -> None:
    script = pathlib.Path(__file__).with_name("rr_integrityd.py")
    subprocess.run(
        [sys.executable, str(script), "scan-once", "--home", str(home), "--policy", str(policy)],
        check=True,
    )


def print_findings(home: pathlib.Path, limit: int) -> None:
    db = home / "findings.db"
    with sqlite3.connect(db) as conn:
        rows = conn.execute(
            """
            SELECT ts, category, severity, subject, payload_json
            FROM findings
            ORDER BY id DESC
            LIMIT ?
            """,
            (limit,),
        ).fetchall()
    for ts, category, severity, subject, payload_json in rows:
        payload = json.loads(payload_json)
        print(f"[{ts}] {severity.upper()} {category} {subject}")
        print(json.dumps(payload, indent=2, sort_keys=True))


def baseline_status(home: pathlib.Path) -> None:
    db = home / "findings.db"
    with sqlite3.connect(db) as conn:
        rows = conn.execute("SELECT category, updated_at FROM baselines ORDER BY category").fetchall()
    for category, updated_at in rows:
        print(f"{category}: {updated_at}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["scan-once", "findings", "baseline-status"])
    parser.add_argument("--home", default="/var/db/rr-integrity")
    parser.add_argument(
        "--policy",
        default=str(pathlib.Path.home() / "Projects" / "rational-reserve" / "config" / "rr-policy.toml"),
    )
    parser.add_argument("--limit", type=int, default=20)
    args = parser.parse_args(argv)

    home = pathlib.Path(args.home)
    policy = pathlib.Path(args.policy)
    if args.command == "scan-once":
        run_scan(home, policy)
    elif args.command == "findings":
        print_findings(home, args.limit)
    else:
        baseline_status(home)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
