# Project Structure - Rational Reserve

**Last Updated**: 2025-12-24
**Version**: 0.2.0
**Status**: Clean and organized

---

## Directory Structure

```
agentxfoundry/
├── rr/                          # RATIONAL RESERVE CORE SYSTEM
│   ├── __init__.py             # Package exports
│   ├── models.py               # Data models (Mission, Task, Agent, etc.)
│   ├── rr_agent.py             # Military-structured agent
│   ├── agent_factory.py        # Agent spawning and formations
│   ├── general_mind.py         # General cognitive framework
│   ├── mission_planner.py      # Autonomous planning engine
│   ├── swarm_orchestrator.py  # Swarm lifecycle management
│   ├── c2_router.py            # Command & control routing
│   └── api_routes.py           # FastAPI REST endpoints
│
├── utils/                       # SHARED UTILITIES
│   ├── base_agent.py           # Abstract agent base class
│   ├── security.py             # JWT auth, encryption
│   ├── self_preservation.py    # Threat detection, self-healing
│   └── recursion_engine.py     # Recursive processing engine
│
├── llm_integrations/            # LLM CLIENT INTEGRATIONS
│   └── clients.py              # Unified LLM wrapper (OpenAI, Anthropic, etc.)
│
├── agents/                      # LEGACY AGENTS (still integrated)
│   └── predictive/             # Predictive analytics agent
│       ├── __init__.py
│       └── predictive_agent.py
│
├── secrets/                     # API KEYS (gitignored)
│   └── (API key files)
│
├── legacy/                      # ARCHIVED COMPONENTS
│   ├── README.md               # Legacy components documentation
│   ├── index.ts                # Genkit demo
│   ├── package.json            # Node.js dependencies
│   ├── package-lock.json       # Locked versions
│   ├── Jasteri.sh              # J* language utility
│   ├── jstar_output/           # J* output files
│   ├── public/                 # Standalone web app
│   └── v2_nf_components.json   # Legacy config
│
├── venv/                        # VIRTUAL ENVIRONMENT (broken, ignored)
├── venv_new/                    # WORKING VIRTUAL ENVIRONMENT
│
├── main.py                      # FASTAPI APPLICATION (RR integrated)
├── requirements.txt             # Python dependencies
├── test_rr_phase1.py           # RR test suite
│
├── Dockerfile                   # Docker containerization
├── docker-compose.yml           # Docker compose config
│
├── README.md                    # Project overview and quick start
├── SESSION_HANDOFF.md          # Session continuity document
├── CHANGELOG.md                # Complete version history
├── CLAUDE.md                   # Development guidance
├── SECURITY.md                 # Security policy
│
├── REQUIREMENTS_RR.md          # Full system requirements
├── REQUIREMENTS_PHASE1.md      # Phase 1 specifications
├── RR_PHASE1_COMPLETE.md       # Phase 1 completion summary
├── RR_PHASE2-3_COMPLETE.md     # Phase 2-3 completion summary
├── PROJECT_STRUCTURE.md        # This file
│
├── .gitignore                  # Git ignore rules
└── __init__.py                 # Root package init
```

---

## Core Components (Active)

### Rational Reserve System (rr/)

**Purpose**: Military-structured autonomous swarm agent system

**Key Files**:
- `models.py`: All data structures (12 ranks, 16 MOS, Mission, Task, etc.)
- `rr_agent.py`: Agent with rank/MOS and cognitive framework integration
- `general_mind.py`: Hardcoded strategic thinking (recursive, reactive, predictive)
- `mission_planner.py`: Autonomous task decomposition and planning
- `swarm_orchestrator.py`: Swarm lifecycle and autonomous execution
- `agent_factory.py`: Spawn agents and military formations
- `c2_router.py`: Command chain communications (orders, SITREPs, CASREPs)
- `api_routes.py`: REST API endpoints

### Supporting Infrastructure

**utils/**: Shared utilities used by RR and legacy systems
- Security (JWT, encryption)
- Base agent framework
- Recursion engine
- Self-preservation

**llm_integrations/**: LLM client wrappers
- Unified interface for OpenAI, Anthropic, Google, Groq, DeepSeek, etc.

**agents/**: Legacy agents still integrated in main.py
- Predictive analytics agent (LSTM-based)

### Application Entry Point

**main.py**: FastAPI application
- Initializes RR system (AgentFactory, SwarmOrchestrator, C2Router)
- Includes RR routes under `/rr/*`
- Maintains legacy predictive agent endpoint
- Runs on port 8000

---

## Documentation Files

### Primary Documentation

| File | Purpose | When to Read |
|------|---------|--------------|
| `README.md` | Project overview, quick start, API reference | First time setup |
| `SESSION_HANDOFF.md` | Current state, session continuity | Every new session |
| `CHANGELOG.md` | Complete version history | Understanding changes |
| `PROJECT_STRUCTURE.md` | This file - organization guide | Understanding layout |

### Technical Specifications

| File | Purpose | When to Read |
|------|---------|--------------|
| `REQUIREMENTS_RR.md` | Full system requirements (all phases) | Deep dive on design |
| `REQUIREMENTS_PHASE1.md` | Phase 1 specifications | Understanding foundation |
| `RR_PHASE1_COMPLETE.md` | Phase 1 completion summary | Phase 1 review |
| `RR_PHASE2-3_COMPLETE.md` | Phase 2-3 completion summary | Current capabilities |

### Development Guidance

| File | Purpose | When to Read |
|------|---------|--------------|
| `CLAUDE.md` | Claude Code development patterns | Contributing code |
| `SECURITY.md` | Security policy | Security review |

---

## Archived Components (legacy/)

**Purpose**: Preserve separate systems not required for RR operation

**Contents**:
- Genkit TypeScript demo
- Jasteri language utility
- Standalone web frontend
- Legacy configurations

**See**: `legacy/README.md` for details

---

## File Counts

**Active Python Files**: ~25
**RR Core Files**: 9 (in rr/)
**Documentation Files**: 9
**Legacy Files**: 7+ (archived)
**Total Lines (RR Core)**: ~3,500
**Total Lines (Including Utils)**: ~5,000

---

## Import Dependencies

### RR System Dependencies:

```
main.py
  └─ rr/
      ├─ models.py (independent)
      ├─ rr_agent.py
      │   ├─ utils/base_agent.py
      │   ├─ models.py
      │   └─ general_mind.py
      ├─ general_mind.py (independent)
      ├─ mission_planner.py
      │   ├─ models.py
      │   └─ general_mind.py
      ├─ agent_factory.py
      │   ├─ rr_agent.py
      │   └─ models.py
      ├─ swarm_orchestrator.py
      │   ├─ agent_factory.py
      │   ├─ mission_planner.py
      │   └─ models.py
      ├─ c2_router.py
      │   ├─ agent_factory.py
      │   └─ models.py
      └─ api_routes.py
          └─ swarm_orchestrator.py
```

**External Dependencies**:
- `llm_integrations/clients.py` (LLM clients)
- `utils/security.py` (JWT authentication)
- `utils/base_agent.py` (Agent base class)

**No dependencies on legacy/ directory**

---

## Testing

**Test Files**:
- `test_rr_phase1.py`: RR system functional tests

**Test Coverage**:
- Agent creation (all ranks/MOS)
- Swarm lifecycle
- Mission execution (autonomous mode)
- C2 communications

---

## Deployment

**Docker**:
- `Dockerfile`: Python 3.11 container with RR system
- `docker-compose.yml`: Service orchestration

**Requirements**:
- Python 3.9+
- Dependencies in `requirements.txt`
- API keys in `secrets/` or environment variables

**Run**:
```bash
python main.py  # Development
docker-compose up  # Production
```

---

## Git Ignore

**.gitignore includes**:
- `secrets/` (API keys)
- `venv/`, `venv_new/` (virtual environments)
- `__pycache__/`, `*.pyc` (Python cache)
- `.DS_Store` (macOS files)
- `jstar_output/` (now in legacy/)

---

## Key Principles

1. **RR Core Isolation**: All RR code in `rr/` directory
2. **No Legacy Dependencies**: RR does not depend on files in `legacy/`
3. **Clear Documentation**: Each major component documented
4. **Session Continuity**: `SESSION_HANDOFF.md` for efficient context transfer
5. **Organized Archive**: Unrelated code preserved but separated

---

## For New Sessions

1. Read `SESSION_HANDOFF.md`
2. Check `CHANGELOG.md` if needed
3. Refer to this file for structure
4. All active code is outside `legacy/`

---

**Status**: Clean, organized, production-ready
**Last Refactored**: 2025-12-24
