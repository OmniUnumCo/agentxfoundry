#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
mkdir -p "$LAUNCH_AGENTS_DIR"

install -m 0644 "$REPO_ROOT/adapters/launchd/com.user.rr.listener-scan.plist" \
    "$LAUNCH_AGENTS_DIR/com.user.rr.listener-scan.plist"
install -m 0644 "$REPO_ROOT/adapters/launchd/com.user.rr.integrity-scan.plist" \
    "$LAUNCH_AGENTS_DIR/com.user.rr.integrity-scan.plist"

mkdir -p "$HOME/.rational-reserve/guard"
if [[ ! -f "$HOME/.rational-reserve/guard/enforce-targets.txt" ]]; then
    cat > "$HOME/.rational-reserve/guard/enforce-targets.txt" <<'EOF'
# Optional exact process names or path fragments to enforce against.
# Enforcement only applies to user-owned processes launched from inside $HOME.
# Apple/system-owned processes are never killed.
EOF
fi

launchctl unload "$LAUNCH_AGENTS_DIR/com.user.rr.listener-scan.plist" 2>/dev/null || true
launchctl unload "$LAUNCH_AGENTS_DIR/com.user.rr.integrity-scan.plist" 2>/dev/null || true
launchctl load "$LAUNCH_AGENTS_DIR/com.user.rr.listener-scan.plist"
launchctl load "$LAUNCH_AGENTS_DIR/com.user.rr.integrity-scan.plist"

printf 'Installed RR guard LaunchAgents in %s\n' "$LAUNCH_AGENTS_DIR"
printf 'Audit events will be written to ~/.rational-reserve/logs/guard-events.jsonl\n'
