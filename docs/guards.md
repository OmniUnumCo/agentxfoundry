# RR Guard Design

These guards are a replacement for the earlier `listener-guard` and `integrity-monitor`
experiments that were installed as root `LaunchDaemons`.

## Design goals

- No root background processes.
- No `while true` shell loops.
- No repeated `lsof` + `ps` fork storms.
- No killing Apple-owned processes.
- Audit-first behavior with narrow, explicit enforcement.

## Components

### `rr-sentinel.sh listener-scan`

- Scheduled by `launchd` every 30 seconds as a **user LaunchAgent**.
- Captures non-loopback TCP listeners.
- Stores a baseline in `~/.rational-reserve/guard/listener-baseline.txt`.
- Logs newly observed listeners to `~/.rational-reserve/logs/guard-events.jsonl`.
- Optional enforcement is limited to:
  - user-owned processes,
  - executables launched from inside `$HOME`,
  - entries explicitly listed in `~/.rational-reserve/guard/enforce-targets.txt`.

### `rr-sentinel.sh integrity-scan`

- Scheduled by `launchd` every 5 minutes as a **user LaunchAgent**.
- Hashes the user/system launchd plist inventory plus RR binaries.
- Stores a baseline in `~/.rational-reserve/guard/integrity-baseline.txt`.
- Logs additions and removals to `~/.rational-reserve/logs/guard-events.jsonl`.

## Why this is safer

The previous daemons ran as root with `RunAtLoad + KeepAlive` and enforced policy by
killing any listener that failed a simplistic ancestry test. That approach is unsafe on
modern macOS because Apple desktop services such as `ControlCenter` legitimately open
listeners and can be relaunched by `launchd`, creating a kill/restart loop.

This replacement keeps the monitoring concept but changes the operating model:

- `launchd` does the scheduling, so there is no hot loop.
- The default mode is observation, not termination.
- Enforcement is opt-in and scoped to explicit user-owned targets.
- Apple processes and system binaries are out of scope by design.

## Activation

```bash
chmod +x ~/Projects/rational-reserve/tools/guards/rr-sentinel.sh
chmod +x ~/Projects/rational-reserve/adapters/install-guards.sh
~/Projects/rational-reserve/adapters/install-guards.sh
```

## Notes

- These are **not** a general-purpose host firewall.
- They are intended as a lightweight local audit layer for agent experiments.
- If you want stronger enforcement, use proper OS-native controls such as the macOS
  application firewall, PF rules, or explicit sandboxing around untrusted workloads.
