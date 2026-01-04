# Project Summary

AgentX Foundry is a dual-track AI agent platform: a FastAPI-based Python backend powering multi-LLM autonomous agents and a Genkit/TypeScript demo for character generation. The repository also ships a custom J* (Jasteri) lexer and signing flow for secure code handling.

## Backend (Python)
- FastAPI entry at `main.py` with rr/* autonomous swarm endpoints.
- Agents inherit from `utils/base_agent.py`; recursion and self-preservation are in `utils/recursion_engine.py` and `utils/self_preservation.py`.
- LLM access unified in `llm_integrations/clients.py` (Anthropic + Ollama, plus other providers when keys are present).
- Rational Reserve swarm lives under `rr/` with mission planning, orchestration, and database persistence (schema in `rr/db_schema.sql`, operations in `rr/db_persistence.py`).

## Frontend / Genkit
- Legacy Genkit demo in `legacy/` for D&D-style character generation via Gemini.

## Operational Notes
- Secrets are loaded from `secrets/` (gitignored). Critical rules: no emojis, no `--pretty=format`.
- Primary virtual environment is `agent_env/` (Python 3.14). `venv_new/` contents are being merged into `agent_env` per user request on 2025-12-27.
- FastAPI default port: 8000. Tests: `python test_rr_phase1.py`.

