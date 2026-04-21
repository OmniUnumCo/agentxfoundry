#!/usr/bin/env bash
# install.sh -- wire Rational Reserve into every supported coding agent.
#
# What this does:
#   1. Copies doctrine/ into ~/.rational-reserve/doctrine/ (version-pinned)
#   2. Copies skills/ into each supported host's skill directory
#   3. Prints per-host MCP config snippets for the user to add (idempotent
#      config merging is error-prone -- we do not attempt to edit existing
#      config files automatically)
#
# Targets:
#   - Claude Code    (~/.claude/skills/)
#   - Codex CLI      (~/.codex/skills/ if present)
#   - Qwen-Code      (~/.qwen/skills/)
#   - OpenCode       (~/.config/opencode/skills/)
#   - Mistral Vibe   (~/.vibe/skills/)
#
# Safe to re-run. Existing skill directories are merged, not replaced.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

RR_HOME="${RR_HOME:-$HOME/.rational-reserve}"

msg() { printf '\033[1;36m[rr-install]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[rr-install]\033[0m %s\n' "$*" >&2; }

# ---------------------------------------------------------------------------
# 1. Runtime layout
# ---------------------------------------------------------------------------
msg "ensuring runtime directories under $RR_HOME"
mkdir -p "$RR_HOME/state" "$RR_HOME/run" "$RR_HOME/logs"

msg "copying doctrine to $RR_HOME/doctrine (version-pinned)"
rm -rf "$RR_HOME/doctrine"
cp -R "$REPO_ROOT/doctrine" "$RR_HOME/doctrine"

# ---------------------------------------------------------------------------
# 1b. Build and install Rust release binaries
# ---------------------------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
    msg "building Rust release binaries"
    (cd "$REPO_ROOT" && cargo build --release)
    mkdir -p "$RR_HOME/bin"
    cp "$REPO_ROOT/target/release/rr-daemon" "$RR_HOME/bin/rr-daemon"
    cp "$REPO_ROOT/target/release/rr-mcp" "$RR_HOME/bin/rr-mcp"
    chmod +x "$RR_HOME/bin/rr-daemon" "$RR_HOME/bin/rr-mcp"
    msg "installed rr-daemon and rr-mcp to $RR_HOME/bin/"
else
    warn "cargo not found -- skipping Rust build (binaries must be pre-built)"
fi

# ---------------------------------------------------------------------------
# 1c. Build and run agent compiler
# ---------------------------------------------------------------------------
if command -v go >/dev/null 2>&1; then
    msg "building rr-compile-agents"
    (cd "$REPO_ROOT/tools/compile-agents" && go build -o "$RR_HOME/bin/rr-compile-agents" .)
    msg "compiling agent definitions from doctrine"
    "$RR_HOME/bin/rr-compile-agents"
else
    warn "go not found -- skipping agent compilation (run rr-compile-agents manually)"
fi

# ---------------------------------------------------------------------------
# 1d. Symlink agents into host agent directories
# ---------------------------------------------------------------------------
link_agents_to() {
    local dest="$1"
    local label="$2"
    if [ ! -d "$(dirname "$dest")" ]; then
        warn "$label: parent directory $(dirname "$dest") does not exist -- skipping agents"
        return 0
    fi
    mkdir -p "$dest"
    local count=0
    for agent in "$RR_HOME"/agents/rr-*.md; do
        [ -f "$agent" ] || continue
        ln -sf "$agent" "$dest/$(basename "$agent")"
        count=$((count + 1))
    done
    msg "$label: symlinked $count agent files to $dest"
}

link_agents_to "$HOME/.claude/agents"            "Claude Code"
link_agents_to "$HOME/.codex/agents"             "Codex CLI"
link_agents_to "$HOME/.qwen/agents"              "Qwen-Code"
link_agents_to "$HOME/.config/opencode/agents"   "OpenCode"
link_agents_to "$HOME/.vibe/agents"              "Mistral Vibe"

# ---------------------------------------------------------------------------
# 2. Per-host skill installation
# ---------------------------------------------------------------------------
install_skills_to() {
    local dest="$1"
    local label="$2"
    if [ ! -d "$(dirname "$dest")" ]; then
        warn "$label: parent directory $(dirname "$dest") does not exist -- skipping"
        return 0
    fi
    mkdir -p "$dest"
    msg "$label: copying skill pack to $dest"
    for skill_dir in "$REPO_ROOT"/skills/*/; do
        local name
        name="$(basename "$skill_dir")"
        mkdir -p "$dest/$name"
        cp "$skill_dir/SKILL.md" "$dest/$name/SKILL.md"
    done
}

install_skills_to "$HOME/.claude/skills"              "Claude Code"
install_skills_to "$HOME/.codex/skills"               "Codex CLI"
install_skills_to "$HOME/.qwen/skills"                "Qwen-Code"
install_skills_to "$HOME/.config/opencode/skills"     "OpenCode"
install_skills_to "$HOME/.vibe/skills"                "Mistral Vibe"

# ---------------------------------------------------------------------------
# 3. Primer files
# ---------------------------------------------------------------------------
copy_primer() {
    local dest="$1"
    local src="$2"
    local label="$3"
    if [ ! -d "$(dirname "$dest")" ]; then
        warn "$label: parent directory missing -- skipping primer"
        return 0
    fi
    msg "$label: installing primer $src -> $dest"
    cp "$REPO_ROOT/adapters/$src" "$dest"
}

copy_primer "$HOME/.claude/CLAUDE.md" "CLAUDE.md" "Claude Code"
copy_primer "$HOME/.codex/AGENTS.md" "AGENTS.md" "Codex CLI"
copy_primer "$HOME/.qwen/AGENTS.md" "AGENTS.md" "Qwen-Code"
copy_primer "$HOME/.config/opencode/AGENTS.md" "AGENTS.md" "OpenCode"
copy_primer "$HOME/.vibe/AGENTS.md" "AGENTS.md" "Mistral Vibe"

# ---------------------------------------------------------------------------
# 4. MCP config snippets
# ---------------------------------------------------------------------------
cat <<EOF

$(msg 'MCP configuration snippets')

The installer does not edit your host agents' config files automatically --
please add the following snippets manually to avoid clobbering unrelated
settings. Each file is idempotent once added.

  Claude Code    : $REPO_ROOT/adapters/mcp-configs/claude-code.mcp.json
                   (copy to ~/.claude/.mcp.json or merge into a project .mcp.json)

  Codex CLI      : $REPO_ROOT/adapters/mcp-configs/codex.config.toml.snippet
                   (append to ~/.codex/config.toml)

  Qwen-Code      : $REPO_ROOT/adapters/mcp-configs/qwen.settings.json.snippet
                   (merge into ~/.qwen/settings.json)

  OpenCode       : $REPO_ROOT/adapters/mcp-configs/opencode.json.snippet
                   (merge into ~/.config/opencode/opencode.json)

  Mistral Vibe   : $REPO_ROOT/adapters/mcp-configs/vibe.config.toml.snippet
                   (append to ~/.vibe/config.toml)

$(msg 'installation complete')
EOF
