# Module documentation index

Each subsystem below has three documents — Requirements, Design Specification, and Technical Specification — per the project's documentation standard.

## Subsystems

| Module | Requirements | Design | Technical |
|---|---|---|---|
| Civilian Reserve (CR) | [REQUIREMENTS](civilian-reserve/REQUIREMENTS.md) | [DESIGN](civilian-reserve/DESIGN.md) | [TECHNICAL](civilian-reserve/TECHNICAL.md) |
| Deterministic LLM Substrate | [REQUIREMENTS](llm-substrate/REQUIREMENTS.md) | [DESIGN](llm-substrate/DESIGN.md) | [TECHNICAL](llm-substrate/TECHNICAL.md) |
| Mission Orchestration | [REQUIREMENTS](mission-orchestration/REQUIREMENTS.md) | [DESIGN](mission-orchestration/DESIGN.md) | [TECHNICAL](mission-orchestration/TECHNICAL.md) |
| Predictive Agent | [REQUIREMENTS](predictive-agent/REQUIREMENTS.md) | [DESIGN](predictive-agent/DESIGN.md) | [TECHNICAL](predictive-agent/TECHNICAL.md) |
| HTTP Surface | [REQUIREMENTS](http-surface/REQUIREMENTS.md) | [DESIGN](http-surface/DESIGN.md) | [TECHNICAL](http-surface/TECHNICAL.md) |

## Reading order

1. **Requirements** — what the subsystem must do. Start here to understand the user-facing contract.
2. **Design Specification** — architecture, data flows, key decisions and rationale. Start here to understand *why* the code is shaped the way it is.
3. **Technical Specification** — implementation detail, types, algorithms, file paths, test coverage. Start here when you're about to edit.

## Cross-subsystem dependencies

```
  Civilian Reserve ---\
                      |
  Mission Orch.   ----+---> LLM Substrate  (all LLM calls flow through DeterministicLLMClient)
                      |
  Predictive Agent ---/
                      |
  HTTP Surface    ----+---> Mission Orch. (execute / disband)
                           \--> Civilian Reserve (stats, rosters -- future)
```

## Original Python sources

Three of the five subsystems are direct ports from `OmniUnumCo/agentxfoundry:main`:

| Rust subsystem | Python source |
|---|---|
| LLM Substrate (providers) | `llm_integrations/clients.py` |
| Mission Orchestration | `rr/general_mind.py`, `rr/mission_planner.py`, `rr/swarm_orchestrator.py` |
| HTTP Surface | `rr/api_routes.py` |
| Predictive Agent | `agents/predictive/predictive_agent.py` (LSTM replaced with deterministic OLS) |
| Civilian Reserve | new; no Python precursor |

## Module boundaries (summary)

- **Civilian Reserve** — new agent class parallel to military ranks. Contracts, not orders.
- **LLM Substrate** — every LLM call across the crate ultimately flows through `DeterministicLLMClient` wrapping a concrete provider.
- **Mission Orchestration** — takes a swarm + objective and turns it into executed tasks with an auditable trail + AAR.
- **Predictive Agent** — deterministic forecasting + multi-LLM analysis ensemble.
- **HTTP Surface** — REST alternative to the MCP stdio shim; same state, different transport.
