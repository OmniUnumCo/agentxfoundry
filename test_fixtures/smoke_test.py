"""End-to-end smoke test: spawn a real daemon subprocess, deploy a swarm,
verify SQLite state via the daemon's own iter_events helper.

Runs against the *real* ~/.rational-reserve/ -- does not isolate state. Useful
as a one-shot manual sanity check after install. Should NOT be run in CI
without first cleaning out ~/.rational-reserve/state/rr.db.
"""

from __future__ import annotations

import json
import os
import sqlite3
import time
from pathlib import Path
from uuid import uuid4

from rr_runtime.config import load_config
from rr_runtime.daemon import connect, ensure_daemon, is_daemon_alive


def rpc(file, method: str, params: dict | None = None) -> dict:
    request = {"id": str(uuid4()), "method": method, "params": params or {}}
    file.write((json.dumps(request) + "\n").encode("utf-8"))
    file.flush()
    raw = file.readline()
    return json.loads(raw.decode("utf-8"))


def main() -> None:
    print("[smoke] ensuring daemon is running...")
    paths = ensure_daemon()
    print(f"[smoke] daemon socket: {paths.socket_path}")
    print(f"[smoke] daemon pid file: {paths.pid_file}")
    assert is_daemon_alive(paths), "daemon should be alive after ensure_daemon"

    sock = connect(paths)
    file = sock.makefile("rwb")

    print("[smoke] ping...")
    pong = rpc(file, "ping")
    print(f"[smoke]   {pong['result']}")
    assert pong["result"]["pong"] is True

    print("[smoke] deploying squad with primary_mos=12B...")
    deploy = rpc(
        file,
        "rr_deploy_swarm",
        {"formation": "squad", "primary_mos": "12B", "swarm_name": "smoke-squad"},
    )
    print(f"[smoke]   swarm_id={deploy['result']['swarm_id']}")
    print(f"[smoke]   agents={deploy['result']['agent_count']}")
    swarm_id = deploy["result"]["swarm_id"]
    assert deploy["result"]["agent_count"] == 8

    print("[smoke] querying status...")
    status = rpc(file, "rr_status", {"swarm_id": swarm_id})
    ranks = sorted({a["rank"] for a in status["result"]["agents"]})
    print(f"[smoke]   ranks present: {ranks}")
    assert "GEN" in ranks
    assert "SGT" in ranks

    print("[smoke] querying roster filtered by rank=SPC...")
    roster = rpc(file, "rr_roster", {"rank": "SPC"})
    print(f"[smoke]   specialist count: {roster['result']['count']}")
    assert roster["result"]["count"] >= 4

    print("[smoke] querying SQLite event log directly...")
    db_path = paths.db_path
    conn = sqlite3.connect(str(db_path))
    cur = conn.cursor()
    cur.execute("SELECT COUNT(*) FROM mission_events")
    event_count = cur.fetchone()[0]
    print(f"[smoke]   total events recorded: {event_count}")

    cur.execute(
        "SELECT event_type, COUNT(*) FROM mission_events GROUP BY event_type ORDER BY 2 DESC"
    )
    print("[smoke]   events by type:")
    for row in cur.fetchall():
        print(f"[smoke]     {row[0]}: {row[1]}")

    cur.execute(
        "SELECT data_json FROM current_state WHERE entity_type='swarm' AND entity_id=?",
        (swarm_id,),
    )
    swarm_row = cur.fetchone()
    assert swarm_row is not None, "swarm projection should exist"
    swarm_doc = json.loads(swarm_row[0])
    print(f"[smoke]   projection name: {swarm_doc['name']}")

    conn.close()
    file.close()
    sock.close()

    print("[smoke] OK ok")


if __name__ == "__main__":
    main()
