# AgentX Foundry - Rational Reserve

**Version**: 0.2.0 (Phase 2-3 Complete)
**Status**: Production Ready
**Motto**: *"Many minds, one mission. Circular, reactive, prescient."*

---

## Quick Start

**IMPORTANT FOR NEW SESSIONS**: Read `SESSION_HANDOFF.md` first for complete current state.

### 1. Install Dependencies

```bash
pip install -r requirements.txt
```

### 2. Configure API Keys

Create files in `secrets/` directory or set environment variables:
- `ANTHROPIC_API_KEY` (required for autonomous planning)
- `OPENAI_API_KEY`
- `GOOGLE_API_KEY`
- Other LLM keys as needed

### 3. Run Server

```bash
python main.py
# Server: http://localhost:8000
# RR Endpoints: /rr/*
```

### 4. Test System

```bash
# Quick test
curl "http://localhost:8000/rr/"

# Create and execute mission
curl -X POST "http://localhost:8000/rr/mission/quick?objective=Write%20fibonacci%20function" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

---

## What Is This?

The Rational Reserve is an **autonomous military-structured swarm agent system** where AI agents with military ranks and specialties work together to accomplish complex tasks.

### Key Features

- **12 Military Ranks**: Private → General hierarchy
- **16 MOS Specialties**: Intelligence, Operations, Support, Special Ops
- **Autonomous Planning**: Generals decompose missions and delegate to specialists
- **General Cognitive Framework**: Recursive, Reactive, Predictive thinking (hardcoded)
- **Dependency Management**: Tasks execute in correct order
- **Adaptive Execution**: Pivots when conditions change
- **Predictive Foresight**: Plans three moves ahead

### How It Works

1. **You**: Provide high-level objective
2. **General**: Analyzes using cognitive framework (circular, reactive, predictive)
3. **Mission Planner**: Decomposes into task graph
4. **Swarm**: Specialists execute tasks by MOS
5. **Result**: Mission completed autonomously

---

## Documentation

**Start Here**:
- `SESSION_HANDOFF.md` - Current state and session context (NEW SESSIONS START HERE)
- `PROJECT_STRUCTURE.md` - Project organization and file layout
- `CHANGELOG.md` - Version history and changes

**Specifications**:
- `REQUIREMENTS_RR.md` - Full system requirements (all phases)
- `REQUIREMENTS_PHASE1.md` - Phase 1 specifications

**Completion Summaries**:
- `RR_PHASE1_COMPLETE.md` - Phase 1 foundation
- `RR_PHASE2-3_COMPLETE.md` - Autonomous operations

**Project Context**:
- `CLAUDE.md` - Development guidance and patterns

**Archived Components**:
- `legacy/` - Genkit demo, Jasteri utility, web frontend (see legacy/README.md)

---

## Architecture

```
User Objective
     ↓
General (Strategic Analysis)
  ├─ Recursive Thinking: Iterative refinement
  ├─ Reactive Complex: Instant adaptation
  └─ Predictive Engine: Foresight (T+1, T+2, T+3)
     ↓
Mission Planner (Autonomous Decomposition)
     ↓
Task Graph (Dependencies + MOS Assignment)
     ↓
Swarm Execution (Specialists by MOS)
     ↓
Results (Aggregated + Adaptive)
```

---

## Core Components

| Component | File | Purpose |
|-----------|------|---------|
| Data Models | `rr/models.py` | Mission, Task, Agent structures |
| RR Agent | `rr/rr_agent.py` | Military-structured agent |
| Agent Factory | `rr/agent_factory.py` | Spawn agents and formations |
| General Mind | `rr/general_mind.py` | Cognitive framework (NEW) |
| Mission Planner | `rr/mission_planner.py` | Autonomous planning (NEW) |
| Orchestrator | `rr/swarm_orchestrator.py` | Swarm lifecycle |
| C2 Router | `rr/c2_router.py` | Command & control |
| API Routes | `rr/api_routes.py` | REST endpoints |

---

## API Reference

### Create Mission
```bash
POST /rr/mission/create
{
  "objective": "Build user authentication system",
  "priority": "urgent",
  "swarm_size": "squad",  # fire_team, squad, platoon
  "primary_mos": "12B"    # Combat Engineer
}
```

### Execute Mission
```bash
POST /rr/swarm/{swarm_id}/execute
```

### Check Status
```bash
GET /rr/swarm/{swarm_id}/status
```

### Get Stats
```bash
GET /rr/stats
```

### Quick Mission (Create + Execute + Disband)
```bash
POST /rr/mission/quick?objective=<your objective>
```

---

## General Cognitive Framework

The General's mind is hardcoded with three integrated frameworks:

### 1. Recursive/Iterative Thinking
- Circular reasoning patterns
- Continuous refinement cycles
- Problems solved through iteration, not single-pass
- All strategies evolve

### 2. Reactive Complex
- Instant pivot capability
- SENSE → EVALUATE → DECIDE → ACT
- Pivot types: PARRY (defend), LUNGE (exploit), DISENGAGE (redesign), HOLD (maintain)
- Millisecond adaptation

### 3. Predictive Engine
- Event horizon analysis
- Projects T+1, T+2, T+3 future states
- Optimal positioning calculations
- Telegraphs moves with quantum precision

**All three combined in `GeneralMind.unified_strategic_prompt()`**

---

## Military Structure

### Ranks (12 Total)

**Officers**: GEN, COL, MAJ, CPT, LT
**NCOs**: SGM, MSG, SGT, CPL
**Enlisted**: SPC, PFC, PVT

### MOS Specialties (16 Total)

**Intelligence**: 35F (Analyst), 35L (Counterintel), 35N (SIGINT)
**Operations**: 11B (Infantry), 12B (Engineer), 13B (Artillery), 19D (Scout)
**Support**: 25B (IT), 42A (HR), 88M (Transport), 92G (Logistics)
**Special Ops**: 18B (SF Engineer), 75R (Ranger), 160 (SOAR)

---

## Development

### Phase 1 (Complete)
- Military hierarchy and MOS system
- Agent framework with LLM integration
- Swarm lifecycle management
- REST API

### Phase 2-3 (Complete)
- General cognitive framework (recursive, reactive, predictive)
- Autonomous mission planning
- Task decomposition and delegation
- Dependency-aware execution

### Future (Optional)
- Database persistence
- Parallel async execution
- Multi-swarm operations
- BCI integration framework

---

## Testing

```bash
# Run Phase 1 tests
python test_rr_phase1.py

# Tests autonomous planning, delegation, execution
```

---

## Philosophy

**Everything in life is circular. To adapt, we must be also.**

The Rational Reserve embodies:
- **Circular thinking**: Iterative refinement, not single-pass
- **Reactive adaptation**: Instant pivots to changing conditions
- **Predictive foresight**: Positioning for future states
- **Military precision**: Clear hierarchy and specialization
- **Autonomous operation**: Works without constant supervision

---

## License

See project documentation for licensing information.

---

## Legacy Components

This project also includes:
- TypeScript/Genkit character generation demo (`index.ts`)
- Jasteri language lexer system (`Jasteri.sh`)
- Predictive analytics agent (legacy)

See `CLAUDE.md` for details on legacy systems.

---

**For Development**: Read `SESSION_HANDOFF.md` for current state
**For Usage**: See API Reference above
**For Understanding**: Read `RR_PHASE2-3_COMPLETE.md`
