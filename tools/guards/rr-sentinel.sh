#!/usr/bin/env bash
set -euo pipefail

RR_HOME="${RR_HOME:-$HOME/.rational-reserve}"
STATE_DIR="$RR_HOME/guard"
LOG_DIR="$RR_HOME/logs"
LISTENER_BASELINE="$STATE_DIR/listener-baseline.txt"
INTEGRITY_BASELINE="$STATE_DIR/integrity-baseline.txt"
EVENT_LOG="$LOG_DIR/guard-events.jsonl"
ENFORCE_FILE="$STATE_DIR/enforce-targets.txt"

mkdir -p "$STATE_DIR" "$LOG_DIR"

json_escape() {
    python3 - <<'PY' "$1"
import json, sys
print(json.dumps(sys.argv[1]))
PY
}

now_utc() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

log_event() {
    local event_type="$1"
    local payload="$2"
    printf '{"ts":"%s","event_type":"%s","payload":%s}\n' "$(now_utc)" "$event_type" "$payload" >> "$EVENT_LOG"
}

apple_proc_regex='^(ControlCenter|SystemUIServer|Dock|WindowServer|rapportd|symptomsd|sharingd|mDNSResponder|bluetoothd|locationd|notifyd|cfprefsd|distnoted|logd)$'

collect_listeners() {
    lsof -nP -iTCP -sTCP:LISTEN 2>/dev/null | awk 'NR > 1 {
        name=$1; pid=$2; user=$3; endpoint=$9;
        split(endpoint, parts, "->");
        local_ep=parts[1];
        full=local_ep;
        sub(/:[^:]+$/, "", full);
        if (full == "*" || full == "127.0.0.1" || full == "::1" || full == "[::1]" || full == "localhost") next;
        print name "|" pid "|" user "|" local_ep;
    }' | sort -u
}

resolve_exe_path() {
    local pid="$1"
    local exe=""

    exe="$(lsof -a -p "$pid" -d txt -Fn 2>/dev/null | sed -n 's/^n//p' | head -n 1)"
    if [[ -n "$exe" ]]; then
        printf '%s\n' "$exe"
        return 0
    fi

    ps -o command= -p "$pid" 2>/dev/null | sed 's/^[[:space:]]*//'
}

is_apple_process() {
    local proc_name="$1"
    local exe="$2"
    local base=""

    base="$(basename "${exe%% *}")"
    [[ "$proc_name" =~ $apple_proc_regex ]] && return 0
    [[ "$base" =~ $apple_proc_regex ]] && return 0
    [[ "$exe" == /System/* || "$exe" == /usr/libexec/* || "$exe" == /usr/sbin/* ]] && return 0
    return 1
}

listener_scan() {
    local current_file
    current_file="$(mktemp)"
    collect_listeners > "$current_file"

    if [[ ! -f "$LISTENER_BASELINE" ]]; then
        cp "$current_file" "$LISTENER_BASELINE"
        log_event "listener_baseline_created" "{\"entries\":$(wc -l < "$LISTENER_BASELINE" | tr -d ' ')}"
        rm -f "$current_file"
        return 0
    fi

    local new_entries
    new_entries="$(comm -13 "$LISTENER_BASELINE" "$current_file" || true)"
    if [[ -n "$new_entries" ]]; then
        while IFS='|' read -r proc_name pid user endpoint; do
            [[ -z "${proc_name:-}" ]] && continue
            local exe=""
            exe="$(resolve_exe_path "$pid")"
            if is_apple_process "$proc_name" "$exe"; then
                continue
            fi
            log_event "new_listener" "$(printf '{"process":%s,"pid":%s,"user":%s,"endpoint":%s,"exe":%s}' \
                "$(json_escape "$proc_name")" \
                "$(json_escape "$pid")" \
                "$(json_escape "$user")" \
                "$(json_escape "$endpoint")" \
                "$(json_escape "$exe")")"
            maybe_enforce "$proc_name" "$pid" "$user" "$endpoint" "$exe"
        done <<< "$new_entries"
    fi

    mv "$current_file" "$LISTENER_BASELINE"
}

collect_integrity_snapshot() {
    {
        find "$HOME/Library/LaunchAgents" -maxdepth 1 -type f -name '*.plist' 2>/dev/null | sort | while read -r file; do
            shasum -a 256 "$file"
        done
        find /Library/LaunchAgents -maxdepth 1 -type f -name '*.plist' 2>/dev/null | sort | while read -r file; do
            shasum -a 256 "$file"
        done
        find /Library/LaunchDaemons -maxdepth 1 -type f -name '*.plist' 2>/dev/null | sort | while read -r file; do
            shasum -a 256 "$file"
        done
        find "$RR_HOME/bin" -maxdepth 1 -type f 2>/dev/null | sort | while read -r file; do
            shasum -a 256 "$file"
        done
    } | sort -u
}

integrity_scan() {
    local current_file
    current_file="$(mktemp)"
    collect_integrity_snapshot > "$current_file"

    if [[ ! -f "$INTEGRITY_BASELINE" ]]; then
        cp "$current_file" "$INTEGRITY_BASELINE"
        log_event "integrity_baseline_created" "{\"entries\":$(wc -l < "$INTEGRITY_BASELINE" | tr -d ' ')}"
        rm -f "$current_file"
        return 0
    fi

    local added removed
    added="$(comm -13 "$INTEGRITY_BASELINE" "$current_file" || true)"
    removed="$(comm -23 "$INTEGRITY_BASELINE" "$current_file" || true)"

    [[ -n "$added" ]] && while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        log_event "integrity_added" "$(printf '{"entry":%s}' "$(json_escape "$line")")"
    done <<< "$added"

    [[ -n "$removed" ]] && while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        log_event "integrity_removed" "$(printf '{"entry":%s}' "$(json_escape "$line")")"
    done <<< "$removed"

    mv "$current_file" "$INTEGRITY_BASELINE"
}

maybe_enforce() {
    local proc_name="$1"
    local pid="$2"
    local user="$3"
    local endpoint="$4"
    local exe="$5"

    [[ -f "$ENFORCE_FILE" ]] || return 0
    [[ "$user" == "$USER" ]] || return 0
    [[ "$exe" == "$HOME"* ]] || return 0
    is_apple_process "$proc_name" "$exe" && return 0

    while IFS= read -r target; do
        [[ -z "$target" || "$target" =~ ^# ]] && continue
        if [[ "$exe" == *"$target"* || "$proc_name" == "$target" ]]; then
            kill "$pid" 2>/dev/null || true
            log_event "listener_enforced" "$(printf '{"process":%s,"pid":%s,"endpoint":%s,"exe":%s,"target":%s}' \
                "$(json_escape "$proc_name")" \
                "$(json_escape "$pid")" \
                "$(json_escape "$endpoint")" \
                "$(json_escape "$exe")" \
                "$(json_escape "$target")")"
            break
        fi
    done < "$ENFORCE_FILE"
}

case "${1:-}" in
    listener-scan)
        listener_scan
        ;;
    integrity-scan)
        integrity_scan
        ;;
    baseline)
        collect_listeners > "$LISTENER_BASELINE"
        collect_integrity_snapshot > "$INTEGRITY_BASELINE"
        log_event "manual_baseline_refresh" '{"ok":true}'
        ;;
    *)
        echo "usage: $0 {listener-scan|integrity-scan|baseline}" >&2
        exit 64
        ;;
esac
