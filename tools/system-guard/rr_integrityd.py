#!/usr/bin/env python3
from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
import pathlib
import sqlite3
import subprocess
import sys
import time
from typing import Iterable

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None


DEFAULT_HOME = pathlib.Path(
    os.environ.get("RR_INTEGRITY_HOME", "/var/db/rr-integrity")
)
DEFAULT_POLICY = pathlib.Path(
    os.environ.get(
        "RR_INTEGRITY_POLICY",
        pathlib.Path.home() / "Projects" / "rational-reserve" / "config" / "rr-policy.toml",
    )
)
DEFAULT_DB = "findings.db"
DEFAULT_LOG = "events.jsonl"
SCHEMA = """
CREATE TABLE IF NOT EXISTS scan_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    error TEXT
);

CREATE TABLE IF NOT EXISTS findings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    ts TEXT NOT NULL,
    category TEXT NOT NULL,
    severity TEXT NOT NULL,
    subject TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    FOREIGN KEY(run_id) REFERENCES scan_runs(id)
);

CREATE TABLE IF NOT EXISTS baselines (
    category TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS process_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL,
    ts TEXT NOT NULL,
    pid INTEGER NOT NULL,
    ppid INTEGER NOT NULL,
    uid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    exe TEXT NOT NULL,
    cpu REAL NOT NULL,
    rss_kb INTEGER NOT NULL,
    vsz_kb INTEGER NOT NULL,
    state TEXT NOT NULL,
    etime TEXT NOT NULL,
    command_line TEXT NOT NULL,
    FOREIGN KEY(run_id) REFERENCES scan_runs(id)
);

CREATE TABLE IF NOT EXISTS process_profiles (
    subject TEXT PRIMARY KEY,
    sample_count INTEGER NOT NULL,
    avg_cpu REAL NOT NULL,
    avg_rss_kb REAL NOT NULL,
    max_cpu REAL NOT NULL,
    max_rss_kb INTEGER NOT NULL,
    listener_seen INTEGER NOT NULL DEFAULT 0,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    profile_json TEXT NOT NULL DEFAULT '{}'
);
"""


@dataclasses.dataclass
class Policy:
    listener_interval_seconds: int
    launchd_interval_seconds: int
    enforce: bool
    kill_apple_processes: bool
    trusted_path_prefixes: list[str]
    trusted_process_names: list[str]
    protected_launchd_paths: list[str]
    actionable_path_prefixes: list[str]
    actionable_process_names: list[str]
    cpu_spike_multiplier: float
    rss_spike_multiplier: float
    min_profile_samples: int


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def ensure_layout(home: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    home.mkdir(parents=True, exist_ok=True)
    db_path = home / DEFAULT_DB
    log_path = home / DEFAULT_LOG
    with sqlite3.connect(db_path) as conn:
        conn.executescript(SCHEMA)
    return db_path, log_path


def load_policy(policy_path: pathlib.Path) -> Policy:
    if tomllib is not None:
        with policy_path.open("rb") as fh:
            raw = tomllib.load(fh)
    else:
        raw = parse_simple_toml(policy_path)
    guard = raw.get("guard", {})
    return Policy(
        listener_interval_seconds=int(guard.get("listener_interval_seconds", 60)),
        launchd_interval_seconds=int(guard.get("launchd_interval_seconds", 300)),
        enforce=bool(guard.get("enforce", False)),
        kill_apple_processes=bool(guard.get("kill_apple_processes", False)),
        trusted_path_prefixes=list(guard.get("trusted_path_prefixes", [])),
        trusted_process_names=list(guard.get("trusted_process_names", [])),
        protected_launchd_paths=list(guard.get("protected_launchd_paths", [])),
        actionable_path_prefixes=list(guard.get("actionable_path_prefixes", [])),
        actionable_process_names=list(guard.get("actionable_process_names", [])),
        cpu_spike_multiplier=float(guard.get("cpu_spike_multiplier", 4.0)),
        rss_spike_multiplier=float(guard.get("rss_spike_multiplier", 3.0)),
        min_profile_samples=int(guard.get("min_profile_samples", 5)),
    )


def parse_simple_toml(policy_path: pathlib.Path) -> dict[str, object]:
    current: dict[str, object] | None = None
    data: dict[str, object] = {}
    multiline_key: str | None = None
    multiline_values: list[str] = []

    def parse_value(raw: str) -> object:
        raw = raw.strip()
        if raw in {"true", "false"}:
            return raw == "true"
        if raw.startswith("[") and raw.endswith("]"):
            inner = raw[1:-1].strip()
            if not inner:
                return []
            return [item.strip().strip('"') for item in inner.split(",") if item.strip()]
        if raw.isdigit():
            return int(raw)
        return raw.strip('"')

    for line in policy_path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if multiline_key:
            if stripped == "]":
                assert current is not None
                current[multiline_key] = multiline_values[:]
                multiline_key = None
                multiline_values.clear()
                continue
            multiline_values.append(stripped.rstrip(",").strip().strip('"'))
            continue
        if stripped.startswith("[") and stripped.endswith("]"):
            section = stripped[1:-1]
            current = data.setdefault(section, {})
            continue
        if "=" not in stripped or current is None:
            continue
        key, value = [part.strip() for part in stripped.split("=", 1)]
        if value == "[":
            multiline_key = key
            multiline_values = []
            continue
        current[key] = parse_value(value)
    return data


def sh(cmd: list[str]) -> str:
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        return ""
    return proc.stdout


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def classify_subject(name: str, exe: str, policy: Policy) -> tuple[bool, bool]:
    trusted = any(name == item for item in policy.trusted_process_names) or any(
        exe.startswith(prefix) for prefix in policy.trusted_path_prefixes
    )
    actionable = any(name == item for item in policy.actionable_process_names) or any(
        exe.startswith(prefix) for prefix in policy.actionable_path_prefixes
    )
    return trusted, actionable


def looks_like_apple(exe: str) -> bool:
    return exe.startswith("/System/") or exe.startswith("/usr/libexec/") or exe.startswith("/usr/sbin/")


def collect_launchd_snapshot(policy: Policy) -> dict[str, dict[str, str]]:
    snapshot: dict[str, dict[str, str]] = {}
    for root in policy.protected_launchd_paths:
        root_path = pathlib.Path(root)
        if not root_path.exists():
            continue
        for item in sorted(root_path.glob("*.plist")):
            try:
                snapshot[str(item)] = {
                    "sha256": sha256_file(item),
                    "mtime": str(int(item.stat().st_mtime)),
                }
            except OSError:
                continue
    return snapshot


def listener_rows() -> list[dict[str, str]]:
    output = sh(["lsof", "-nP", "-iTCP", "-sTCP:LISTEN", "-FpcLn"])
    rows: list[dict[str, str]] = []
    current: dict[str, str] = {}
    for line in output.splitlines():
        if not line:
            continue
        key, value = line[0], line[1:]
        if key == "p":
            if current:
                rows.append(current)
            current = {"pid": value}
        elif key == "c":
            current["command"] = value
        elif key == "L":
            current["user"] = value
        elif key == "n":
            current["endpoint"] = value
    if current:
        rows.append(current)
    return rows


def process_rows() -> list[dict[str, str]]:
    output = sh(
        [
            "ps",
            "-axo",
            "pid=,ppid=,uid=,comm=,%cpu=,rss=,vsz=,state=,etime=,command=",
        ]
    )
    rows: list[dict[str, str]] = []
    for line in output.splitlines():
        line = line.rstrip()
        if not line:
            continue
        parts = line.split(None, 9)
        if len(parts) < 10:
            continue
        pid, ppid, uid, comm, cpu, rss, vsz, state, etime, command = parts
        rows.append(
            {
                "pid": pid,
                "ppid": ppid,
                "uid": uid,
                "comm": comm,
                "cpu": cpu,
                "rss": rss,
                "vsz": vsz,
                "state": state,
                "etime": etime,
                "command": command,
            }
        )
    return rows


def resolve_exe(pid: str) -> str:
    out = sh(["lsof", "-a", "-p", pid, "-d", "txt", "-Fn"])
    for line in out.splitlines():
        if line.startswith("n"):
            return line[1:]
    cmd = sh(["ps", "-o", "command=", "-p", pid]).strip()
    return cmd


def collect_listener_snapshot(policy: Policy) -> dict[str, dict[str, str | bool]]:
    snapshot: dict[str, dict[str, str | bool]] = {}
    for row in listener_rows():
        pid = row.get("pid")
        if not pid:
            continue
        exe = resolve_exe(pid)
        command = row.get("command", "")
        endpoint = row.get("endpoint", "")
        trusted, actionable = classify_subject(command, exe, policy)
        snapshot[f"{pid}:{endpoint}"] = {
            "pid": pid,
            "command": command,
            "user": row.get("user", ""),
            "endpoint": endpoint,
            "exe": exe,
            "apple": looks_like_apple(exe),
            "trusted": trusted,
            "actionable": actionable,
        }
    return snapshot


def process_subject(comm: str, exe: str) -> str:
    if exe:
        head = exe.split(" ", 1)[0]
        return head
    return comm


def collect_process_snapshot(policy: Policy) -> dict[str, dict[str, object]]:
    snapshot: dict[str, dict[str, object]] = {}
    for row in process_rows():
        pid = row["pid"]
        exe = resolve_exe(pid)
        comm = row["comm"]
        subject = process_subject(comm, exe)
        trusted, actionable = classify_subject(comm, exe, policy)
        snapshot[pid] = {
            "pid": int(pid),
            "ppid": int(row["ppid"]),
            "uid": int(row["uid"]),
            "comm": comm,
            "exe": exe,
            "cpu": float(row["cpu"]),
            "rss_kb": int(row["rss"]),
            "vsz_kb": int(row["vsz"]),
            "state": row["state"],
            "etime": row["etime"],
            "command_line": row["command"],
            "subject": subject,
            "apple": looks_like_apple(exe),
            "trusted": trusted,
            "actionable": actionable,
        }
    return snapshot


def write_event(log_path: pathlib.Path, event: dict[str, object]) -> None:
    with log_path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(event, sort_keys=True) + "\n")


def load_baseline(conn: sqlite3.Connection, category: str) -> dict[str, object]:
    row = conn.execute(
        "SELECT payload_json FROM baselines WHERE category = ?",
        (category,),
    ).fetchone()
    return json.loads(row[0]) if row else {}


def save_baseline(conn: sqlite3.Connection, category: str, payload: dict[str, object]) -> None:
    conn.execute(
        """
        INSERT INTO baselines(category, payload_json, updated_at)
        VALUES(?, ?, ?)
        ON CONFLICT(category) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at
        """,
        (category, json.dumps(payload, sort_keys=True), utc_now()),
    )


def record_finding(
    conn: sqlite3.Connection,
    log_path: pathlib.Path,
    run_id: int,
    category: str,
    severity: str,
    subject: str,
    payload: dict[str, object],
) -> None:
    ts = utc_now()
    conn.execute(
        """
        INSERT INTO findings(run_id, ts, category, severity, subject, payload_json)
        VALUES(?, ?, ?, ?, ?, ?)
        """,
        (run_id, ts, category, severity, subject, json.dumps(payload, sort_keys=True)),
    )
    write_event(
        log_path,
        {
            "ts": ts,
            "category": category,
            "severity": severity,
            "subject": subject,
            "payload": payload,
        },
    )


def load_process_profile(conn: sqlite3.Connection, subject: str) -> sqlite3.Row | None:
    conn.row_factory = sqlite3.Row
    row = conn.execute(
        """
        SELECT subject, sample_count, avg_cpu, avg_rss_kb, max_cpu, max_rss_kb,
               listener_seen, first_seen_at, last_seen_at, profile_json
        FROM process_profiles
        WHERE subject = ?
        """,
        (subject,),
    ).fetchone()
    conn.row_factory = None
    return row


def save_process_profile(
    conn: sqlite3.Connection,
    subject: str,
    sample: dict[str, object],
    listener_seen: bool,
) -> None:
    now = utc_now()
    existing = load_process_profile(conn, subject)
    if existing is None:
        conn.execute(
            """
            INSERT INTO process_profiles(
                subject, sample_count, avg_cpu, avg_rss_kb, max_cpu, max_rss_kb,
                listener_seen, first_seen_at, last_seen_at, profile_json
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                subject,
                1,
                sample["cpu"],
                sample["rss_kb"],
                sample["cpu"],
                sample["rss_kb"],
                1 if listener_seen else 0,
                now,
                now,
                json.dumps({"comm": sample["comm"], "exe": sample["exe"]}, sort_keys=True),
            ),
        )
        return

    count = int(existing["sample_count"])
    avg_cpu = float(existing["avg_cpu"])
    avg_rss = float(existing["avg_rss_kb"])
    cpu = float(sample["cpu"])
    rss = float(sample["rss_kb"])
    new_count = count + 1
    new_avg_cpu = ((avg_cpu * count) + cpu) / new_count
    new_avg_rss = ((avg_rss * count) + rss) / new_count
    new_max_cpu = max(float(existing["max_cpu"]), cpu)
    new_max_rss = max(int(existing["max_rss_kb"]), int(rss))
    conn.execute(
        """
        UPDATE process_profiles
        SET sample_count = ?, avg_cpu = ?, avg_rss_kb = ?, max_cpu = ?, max_rss_kb = ?,
            listener_seen = ?, last_seen_at = ?, profile_json = ?
        WHERE subject = ?
        """,
        (
            new_count,
            new_avg_cpu,
            new_avg_rss,
            new_max_cpu,
            new_max_rss,
            1 if (int(existing["listener_seen"]) or listener_seen) else 0,
            now,
            json.dumps({"comm": sample["comm"], "exe": sample["exe"]}, sort_keys=True),
            subject,
        ),
    )


def maybe_record_process_anomaly(
    conn: sqlite3.Connection,
    log_path: pathlib.Path,
    run_id: int,
    sample: dict[str, object],
    policy: Policy,
    listener_seen: bool,
) -> None:
    profile = load_process_profile(conn, str(sample["subject"]))
    if profile is None:
        return
    if int(profile["sample_count"]) < policy.min_profile_samples:
        return

    cpu = float(sample["cpu"])
    rss = int(sample["rss_kb"])
    avg_cpu = max(float(profile["avg_cpu"]), 0.1)
    avg_rss = max(float(profile["avg_rss_kb"]), 1.0)
    max_cpu = float(profile["max_cpu"])
    max_rss = int(profile["max_rss_kb"])

    reasons: list[str] = []
    if cpu >= avg_cpu * policy.cpu_spike_multiplier and cpu > max_cpu:
        reasons.append("cpu_spike")
    if rss >= avg_rss * policy.rss_spike_multiplier and rss > max_rss:
        reasons.append("rss_spike")
    if listener_seen and not int(profile["listener_seen"]):
        reasons.append("new_listener_behavior")
    if not reasons:
        return

    severity = "warning"
    if sample["apple"]:
        severity = "info"
    elif sample["actionable"]:
        severity = "critical"
    record_finding(
        conn,
        log_path,
        run_id,
        "process_anomaly",
        severity,
        str(sample["subject"]),
        {
            "reasons": reasons,
            "sample": sample,
            "baseline": {
                "sample_count": int(profile["sample_count"]),
                "avg_cpu": avg_cpu,
                "avg_rss_kb": avg_rss,
                "max_cpu": max_cpu,
                "max_rss_kb": max_rss,
                "listener_seen": bool(profile["listener_seen"]),
            },
        },
    )


def diff_snapshots(
    baseline: dict[str, object],
    current: dict[str, object],
) -> tuple[set[str], set[str]]:
    return set(current).difference(baseline), set(baseline).difference(current)


def scan_once(home: pathlib.Path, policy_path: pathlib.Path) -> None:
    db_path, log_path = ensure_layout(home)
    policy = load_policy(policy_path)
    with sqlite3.connect(db_path) as conn:
        started_at = utc_now()
        cur = conn.execute(
            "INSERT INTO scan_runs(started_at, status) VALUES(?, ?)",
            (started_at, "running"),
        )
        run_id = int(cur.lastrowid)
        try:
            launchd_current = collect_launchd_snapshot(policy)
            launchd_baseline = load_baseline(conn, "launchd")
            added, removed = diff_snapshots(launchd_baseline, launchd_current)
            for path in sorted(added):
                record_finding(
                    conn,
                    log_path,
                    run_id,
                    "launchd",
                    "warning",
                    path,
                    {"change": "added", "entry": launchd_current[path]},
                )
            for path in sorted(removed):
                record_finding(
                    conn,
                    log_path,
                    run_id,
                    "launchd",
                    "critical",
                    path,
                    {"change": "removed", "entry": launchd_baseline[path]},
                )
            save_baseline(conn, "launchd", launchd_current)

            listeners_current = collect_listener_snapshot(policy)
            listeners_baseline = load_baseline(conn, "listeners")
            added, removed = diff_snapshots(listeners_baseline, listeners_current)
            for key in sorted(added):
                entry = listeners_current[key]
                severity = "info"
                if entry["apple"]:
                    severity = "debug"
                elif entry["trusted"]:
                    severity = "info"
                elif entry["actionable"]:
                    severity = "warning"
                else:
                    severity = "warning"
                record_finding(
                    conn,
                    log_path,
                    run_id,
                    "listener",
                    severity,
                    key,
                    {"change": "added", "entry": entry},
                )
            for key in sorted(removed):
                record_finding(
                    conn,
                    log_path,
                    run_id,
                    "listener",
                    "info",
                    key,
                    {"change": "removed", "entry": listeners_baseline[key]},
                )
            save_baseline(conn, "listeners", listeners_current)

            active_listener_pids = {
                str(entry["pid"]) for entry in listeners_current.values() if "pid" in entry
            }
            processes_current = collect_process_snapshot(policy)
            for sample in processes_current.values():
                conn.execute(
                    """
                    INSERT INTO process_samples(
                        run_id, ts, pid, ppid, uid, comm, exe, cpu, rss_kb, vsz_kb,
                        state, etime, command_line
                    ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        run_id,
                        utc_now(),
                        sample["pid"],
                        sample["ppid"],
                        sample["uid"],
                        sample["comm"],
                        sample["exe"],
                        sample["cpu"],
                        sample["rss_kb"],
                        sample["vsz_kb"],
                        sample["state"],
                        sample["etime"],
                        sample["command_line"],
                    ),
                )
                listener_seen = str(sample["pid"]) in active_listener_pids
                maybe_record_process_anomaly(
                    conn, log_path, run_id, sample, policy, listener_seen
                )
                save_process_profile(
                    conn, str(sample["subject"]), sample, listener_seen
                )

            conn.execute(
                "UPDATE scan_runs SET finished_at = ?, status = ? WHERE id = ?",
                (utc_now(), "ok", run_id),
            )
            conn.commit()
        except Exception as exc:  # pragma: no cover
            conn.execute(
                "UPDATE scan_runs SET finished_at = ?, status = ?, error = ? WHERE id = ?",
                (utc_now(), "error", str(exc), run_id),
            )
            conn.commit()
            raise


def daemon(home: pathlib.Path, policy_path: pathlib.Path) -> None:
    policy = load_policy(policy_path)
    interval = min(policy.listener_interval_seconds, policy.launchd_interval_seconds)
    while True:
        scan_once(home, policy_path)
        time.sleep(max(interval, 30))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["scan-once", "daemon"])
    parser.add_argument("--home", default=str(DEFAULT_HOME))
    parser.add_argument("--policy", default=str(DEFAULT_POLICY))
    args = parser.parse_args(argv)

    home = pathlib.Path(args.home)
    policy = pathlib.Path(args.policy)
    if args.command == "scan-once":
        scan_once(home, policy)
        return 0
    daemon(home, policy)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
