# RR System Guard C++

Native C++ implementation of the Rational Reserve system integrity daemon concept.

## Goals

- deterministic compiled binaries
- system-wide process telemetry
- durable baselines and anomaly detection
- no shell-loop polling daemons
- no automatic action against Apple/system services

## Binaries

- `rr-integrityd` — scanner / daemon
- `rrctl` — findings and profile inspection

## Build

```bash
tools/system-guard-cpp/build.sh
```

## Current scope

- collects process inventory using `libproc`
- records per-process samples in SQLite
- maintains per-subject process profiles
- records launchd baseline changes
- emits anomaly findings for CPU/RSS spikes relative to learned profiles

Listener telemetry is intentionally not implemented here yet; the old listener-only
approach was the least trustworthy part of the previous design, so the C++ port starts
with full-process baselining first.
