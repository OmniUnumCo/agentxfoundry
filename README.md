# Rational Reserve

> *"Many minds, one mission."*

Deterministic, compiled Rust for multi-agent orchestration. Ships as three
binaries — a singleton daemon (`rr-daemon`), an MCP stdio shim
(`rr-mcp`), and an HTTP server (`rr-http`) — all operating against one
local state store.

Designed to be invoked from any MCP-capable coding agent (Claude Code,
Codex CLI, Qwen-Code, OpenCode, Mistral Vibe) and from plain HTTP
clients alike.

## Thesis

Every LLM call in Rational Reserve goes through
[`DeterministicLLMClient`](docs/modules/llm-substrate/DESIGN.md):
`temperature=0` is forced, a seed is threaded where the provider
supports it, and responses are cached by BLAKE3 content hash so
repeated calls replay byte-identically. Every prompt is built from
frontmatter-parsed doctrine files with stable ordering — no
timestamps, no UUIDs, no HashMap-iteration drift. Every orchestration
decision — mission planning, task assignment, specialist routing — is
deterministic or explicitly LLM-backed with the LLM wrapped.

The net effect: given a cold cache and a running local Ollama,
two identical calls produce identical output on the first run and
skip the network entirely on the second.

## Two agent classes

**Military.** Rank + MOS + chain-of-command. Generals give orders,
Specialists execute them, SITREPs flow up, FRAGOs flow down,
failures escalate as CASREPs, finished missions emit AARs.

**Civilian Reserve.** Project-specialized contractors that military
agents *hire* for domain expertise. They live outside the chain of
command — they cannot be ordered, only contracted. Contracts address
*firms* (umbrella civilians); firms route internally to their
specialists via a deterministic keyword-rule broker. Full design:
[docs/modules/civilian-reserve/](docs/modules/civilian-reserve/DESIGN.md).

## Architecture

```
+-----------------+  +------------------+   +-----------------+
| Host agents     |  | Browsers / tools |   | CI / operators  |
| (Claude Code,   |  | (REST clients)   |   | (direct socket) |
| Codex, Qwen,..) |  +--------+---------+   +--------+--------+
+--------+--------+           |                      |
         | MCP stdio          | HTTP                 | unix socket
         v                    v                      v
+-----------------+  +------------------+   +-----------------+
| rr-mcp shim     |  | rr-http (axum)   |   | rr-daemon       |
+--------+--------+  +--------+---------+   +--------+--------+
         |                    |                      |
         +--------+-----------+-----------+----------+
                  |
                  v
+------------------------------------------------------------+
|  DaemonState                                               |
|  state_store | doctrine | llm | router | factory | logger  |
+--+------+--------+------+------+------+---------+----------+
   |      |        |      |      |      |
   v      v        v      v      v      v
 models  persis-  c2_    doctrine llm/ swarm_orchestrator
  and     tence   router           broker  mission_planner
 traits  (SQLite  (chain  (YAML    (tier  predictive
         WAL +    of cmd  front-   router general_mind
         civil+   + contr matter   + 4    vectordb
         contr)   + civ    + slug   prov-
                  refuse)  validat- iders)
                           ion)
```

All state mutations flow through the daemon. The event log is the
source of truth; projection tables (`agents`, `swarms`, `missions`,
`civilians`, `contracts`) are derived.

## Binaries

```bash
cargo build --release
#   target/release/rr-daemon
#   target/release/rr-mcp
#   target/release/rr-http
```

| Binary | Transport | Where |
|---|---|---|
| `rr-daemon` | unix socket (line-JSON RPC) | `~/.rational-reserve/run/rr.sock` |
| `rr-mcp` | stdio subprocess (MCP) | spawned per host agent |
| `rr-http` | TCP (axum) | `RR_HTTP_BIND`, default `127.0.0.1:8787` |

All three serve the same `DaemonState`. A mission created over HTTP
is visible to MCP clients and vice versa; `rr-daemon` is the
single writer to the event log.

## LLM provider matrix

Eleven providers covered by four Rust clients:

| Client | Provider | Models (examples) |
|---|---|---|
| `OllamaClient` | Ollama (local) | `llama3.2:3b`, `qwen2.5-coder:7b`, any pulled model |
| | Llama-family via Ollama | `llama3:latest`, `llama3.1:8b-instruct` |
| | DeepSeek-cloud via Ollama | `deepseek-v3.1:671b-cloud` |
| | Qwen-cloud via Ollama | `qwen3-coder:480b-cloud` |
| `AnthropicClient` | Anthropic Claude | `claude-sonnet-4-20250514` |
| `OpenAiCompatClient` | OpenAI | `gpt-4o`, `gpt-4` |
| | Groq | `llama-3.1-70b-versatile`, `mixtral-8x7b-32768` |
| | Mistral / Devstral | `mistral-large`, `devstral-small` |
| | Kimi / Kimi Coder (Moonshot) | `moonshot-v1-128k`, `kimi-k1-8k` |
| | Qwen Coder | `qwen-coder-plus` |
| `GoogleClient` | Google Gemini | `gemini-2.0-flash`, `gemini-pro` |

Routing is tier-driven. A [`config/llm.toml`](config/llm.toml) template
ships with the repo; override with `RR_LLM_CONFIG_PATH`. Every tier
(`strategic` / `tactical` / `supervision` / `execution_guidance` /
`execution`) can point at any provider / model. API keys come from
env: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`,
`MISTRAL_API_KEY`, `KIMI_API_KEY`, `QWEN_API_KEY`, `GOOGLE_API_KEY`,
`OLLAMA_HOST`.

## Civilian Reserve roster

| Kind | Slug | Source |
|---|---|---|
| Independent | [`arena`](doctrine/civilian/flat/arena.md) | `~/Projects/arena` |
| Independent | [`btcwallet`](doctrine/civilian/flat/btcwallet.md) | upstream btcsuite |
| Independent | [`openchoreo`](doctrine/civilian/flat/openchoreo.md) | upstream WSO2 |
| Firm | [`pitchfork`](doctrine/civilian/firms/pitchfork/firm.md) | Settlement-Contract, ZK-KYC, VPS-Isolation, BTC-Custody |
| Firm | [`developer-portal`](doctrine/civilian/firms/developer-portal/firm.md) | Backstage, Gitea, k3d, OpenChoreo-M3-Prep |
| Firm | [`knockout-eda`](doctrine/civilian/firms/knockout-eda/firm.md) | 9 statistical workflows |
| Firm | [`return42`](doctrine/civilian/firms/return42/firm.md) | Morphlex, Jasterish, Custom-LLM |
| Firm | [`mab`](doctrine/civilian/firms/mab/firm.md) | Multimodal-IO, MOA-Reasoning, Recursive-Loop-Closure, Multi-LLM-Adapter |

32 doctrine files total (3 flat + 5 firms + 24 specialists).

## MCP tool surface

Listed by `rr-mcp` on `tools/list`:

**Military:**
- `rr_roster` — list agents (military + civilian) with class + rank + MOS filters
- `rr_deploy_swarm` — deploy a new swarm (single / fire_team / squad / platoon)
- `rr_status` — live status projection for a swarm
- `rr_execute_mission` — plan -> assign -> execute DAG
- `rr_disband` — disband + AAR with contract timelines

**Civilian Reserve:**
- `cr_register_civilian` — register a civilian from its doctrine file
- `cr_list_civilians` — list all registered civilians
- `cr_firm_detail` — inspect a firm (doctrine + specialists + routing rules + open contracts)
- `cr_contract_request` — initiate a contract (military -> firm)
- `cr_contract_accept` — firm-side: accept + assign to specialist
- `cr_contract_sitrep` — submit progress (implicit Accepted -> InProgress on first call)
- `cr_contract_closeout` — close with outcome + deliverables
- `cr_execute_contract` — build prompt from doctrine + brief, call LLM, record SITREP, auto-close

## HTTP surface

Exposed by `rr-http` (axum):

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/` | service identity |
| `POST` | `/mission/create` | create mission + swarm |
| `POST` | `/swarm/{swarm_id}/execute` | plan + assign + run |
| `GET` | `/swarm/{swarm_id}/status` | live projection |
| `POST` | `/swarm/{swarm_id}/disband` | disband + AAR |
| `GET` | `/swarms` | list all swarms |
| `GET` | `/stats` | event + swarm + civilian counts |
| `GET` | `/agents/roster` | full agent list |
| `POST` | `/mission/quick` | create + execute in one call |

## State lives in `~/.rational-reserve/`

```
~/.rational-reserve/
+-- state/rr.db            # SQLite WAL: events + projections
+-- state/llm_cache.db     # BLAKE3-keyed response cache
+-- run/rr.pid             # daemon PID
+-- run/rr.sock            # unix socket
+-- logs/c2.jsonl          # structured JSONL event log
+-- doctrine/              # version-pinned doctrine copy
```

`doctrine_version` stamped on every event row is
`mil-<X.Y.Z>+civ-<A.B.C>` — military and civilian doctrine evolve
independently while keeping exact per-event reproducibility.

## Development

```bash
# Build all three binaries
cargo build --release

# Run every test (including docs; ignores 3 live-Ollama tests by default)
cargo test

# Full determinism + E2E sweep (requires local Ollama + a pulled model)
#   brew install ollama && ollama serve &
#   ollama pull llama3.2:3b
cargo test --test test_live_ollama -- --ignored

# Start the daemon (foreground, dev)
cargo run --bin rr-daemon

# Drive the MCP shim (spawned automatically by host agents)
cargo run --bin rr-mcp

# Serve the HTTP surface
RR_HTTP_BIND=127.0.0.1:8787 cargo run --bin rr-http

# Use a specific LLM config
RR_LLM_CONFIG_PATH=$PWD/config/llm.toml cargo run --bin rr-daemon
```

## Module documentation

Every subsystem has Requirements + Design + Technical docs per
project standard:

- [Civilian Reserve](docs/modules/civilian-reserve/)
- [Deterministic LLM Substrate](docs/modules/llm-substrate/)
- [Mission Orchestration](docs/modules/mission-orchestration/)
- [Predictive Agent](docs/modules/predictive-agent/)
- [HTTP Surface](docs/modules/http-surface/)

See [docs/modules/README.md](docs/modules/README.md) for the index
and cross-subsystem dependency map.

## Optional guard layers

Separate, opt-in integrity-scanning utilities live under `tools/`.
They are not part of the RR daemon and do not share its state:

- `tools/guards/` — user-scoped LaunchAgents for listener /
  persistence audit. See [docs/guards.md](docs/guards.md).
- `tools/system-guard/` — system-wide integrity daemon design (audit-
  first, staged rollout). See [docs/system-integrity-daemon.md](docs/system-integrity-daemon.md).

## Status

**Shipped:**
- Core Rust library (models, traits, persistence, C2 router, doctrine,
  factory, daemon, MCP shim)
- Civilian Reserve: class model, firm/specialist hierarchy, deterministic
  broker, contract lifecycle, 8 MCP tools, 32 authored civilian
  doctrine files
- Deterministic LLM substrate: 4 clients covering 11 providers,
  content-addressed cache, tier router, config/llm.toml example
- Mission orchestration: all 5 General-Mind prompt modes,
  plan/adapt/predict, execute-mission DAG walker, disband + AAR
  with contract timelines
- Predictive agent: closed-form OLS forecaster + multi-LLM ensemble
  (deterministic replacement for the Python LSTM)
- HTTP surface: 8 axum endpoints, `rr-http` binary
- Live-Ollama E2E tests (ignored by default; run explicitly)
- 158 tests passing, 3 live-Ollama ignored, zero failures

**Deferred:**
- ONNX Runtime provider (in-process pinned-weights inference)
- LanceDB persistent vector store (BruteForceStore is the default)
- Retrieval-augmented prompt assembly
- Streaming LLM responses
- PostgreSQL persistence (SQLite is the default; Python prototype
  used PG)

---

*Interfaces are versioned via doctrine files; the daemon's
unix-socket JSON-RPC protocol is considered an internal surface
until v1.0. See [docs/modules/](docs/modules/) for authoritative
subsystem specs.*
