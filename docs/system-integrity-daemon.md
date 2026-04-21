# RR System Integrity Daemon v2

This is a replacement design for the failed `listener-guard` / `integrity-monitor`
experiments.

## Scope

- System-wide visibility
- Root daemon deployment model
- Audit-first operation
- Explicit policy file
- No blind process killing

## Components

### `tools/system-guard-cpp/src/rr_integrityd.cpp`

Native scanner/daemon that:

- baselines `LaunchDaemons` / `LaunchAgents`
- samples all running processes via `libproc`
- records findings in SQLite
- writes JSONL events for downstream review

It is intentionally audit-first by default. Enforcement exists, but stays off until
`enforce = true` in policy.

It also samples **all running processes** on each scan and builds per-subject profiles:

- average CPU
- average RSS
- average thread count
- listener socket activity
- active network socket activity
- historical maxima
- whether listener behavior has been seen before
- whether network behavior has been seen before

That allows the daemon to reason about deviations from normal behavior rather than
making decisions from “this process exists” or “this process opened a port” alone.

### `tools/system-guard-cpp/src/rrctl.cpp`

Operator CLI for:

- `scan-once`
- `runs`
- `findings`
- `profiles`
- `anomalies`
- `inspect <subject>`

### `config/rr-policy.toml`

Separates and tunes:

- trusted Apple/system processes
- trusted infrastructure
- actionable user-space targets
- protected launchd paths
- adaptive anomaly thresholds for CPU, RSS, threads, listeners, and network activity
- confidence thresholds for observe / alert / contain / terminate decisions

## Why this is different

The old design made kill decisions from a single signal: “process opened a listener.”
That is not a viable system integrity model on macOS.

This design instead uses:

- stateful baselines
- process-behavior profiles over time
- explicit trust policy
- durable findings store
- operator review via `rrctl`

## Suggested rollout

1. Run `scan-once` manually for several days.
2. Review findings and tune `rr-policy.toml`.
3. Install the root LaunchDaemon only after false positives are under control.
4. Add constrained enforcement only after the adaptive process profiles are stable and
   understandable across both user and system subjects.
5. Only enable contain/terminate actions after process-profile findings are stable and
   understandable.

## Example usage

```bash
tools/system-guard-cpp/build.sh

tools/system-guard-cpp/build/rr-integrityd scan-once \
  --home /tmp/rr-integrity-test \
  --policy config/rr-policy.toml

tools/system-guard-cpp/build/rrctl findings \
  --home /tmp/rr-integrity-test \
  --limit 20
```
