# Rational Reserve (RR) - Swarm Agent System Requirements

**Version:** 1.0
**Status:** Draft
**Last Updated:** 2025-12-24

---

## 1. Executive Summary

The Rational Reserve (RR), colloquially known as the "swaRRm," is a military-inspired swarm intelligence system where AI agents operate within a hierarchical command structure to accomplish complex tasks through coordinated action. Each agent holds a military rank and specialty, enabling efficient task delegation, parallel execution, and chain-of-command oversight.

---

## 2. System Overview

### 2.1 Vision

Transform AgentX Foundry into a swarm generator capable of:
- Spawning hierarchical agent teams on-demand
- Coordinating multi-agent task execution with military precision
- Self-organizing based on mission requirements
- Providing clear command structure and accountability
- Scaling from small fire teams to battalion-level operations

### 2.2 Core Principles

1. **Chain of Command**: Every agent reports to a superior officer
2. **Specialization**: Agents have Military Occupational Specialties (MOS)
3. **Mission-Oriented**: Swarms form around objectives, disband when complete
4. **Resilience**: NCOs can assume officer duties if needed
5. **Coordination**: Shared intelligence and synchronized operations

---

## 3. Military Hierarchy & Ranks

### 3.1 Officer Ranks (Strategic & Tactical Leadership)

| Rank | Role | Responsibilities |
|------|------|------------------|
| **General (GEN)** | Swarm Commander | Overall mission planning, resource allocation, strategic decisions |
| **Colonel (COL)** | Battalion Commander | Coordinate multiple task forces, manage officer corps |
| **Major (MAJ)** | Operations Officer | Mission decomposition, assign objectives to captains |
| **Captain (CPT)** | Company Commander | Lead specialized teams (recon, assault, support), tactical execution |
| **Lieutenant (LT)** | Platoon Leader | Direct supervision of NCOs and specialists, immediate tactical decisions |

### 3.2 Non-Commissioned Officer Ranks (Execution Leadership)

| Rank | Role | Responsibilities |
|------|------|------------------|
| **Sergeant Major (SGM)** | Senior NCO | Coordinate all enlisted activities, liaison to officers |
| **Master Sergeant (MSG)** | Section Chief | Lead specialized sections (intelligence, logistics, comms) |
| **Sergeant (SGT)** | Team Leader | Direct supervision of specialists, hands-on execution |
| **Corporal (CPL)** | Fire Team Leader | Lead 2-4 specialists in focused tasks |

### 3.3 Enlisted Specialists

| Rank | Role | Responsibilities |
|------|------|------------------|
| **Specialist (SPC)** | Technical Expert | Execute specific tasks (coding, analysis, data processing) |
| **Private First Class (PFC)** | Junior Specialist | Assist specialists, learn and execute simple tasks |
| **Private (PVT)** | Entry Agent | Basic task execution, data gathering, reconnaissance |

---

## 4. Military Occupational Specialties (MOS)

### 4.1 Intelligence (MI)

- **35F - Intelligence Analyst**: Data analysis, pattern recognition, threat assessment
- **35L - Counterintelligence**: Security validation, threat detection, anomaly identification
- **35N - SIGINT Analyst**: Signal processing, log analysis, communication monitoring

### 4.2 Operations (OPS)

- **11B - Infantry (Core Execution)**: General-purpose task execution, adaptable workers
- **12B - Combat Engineer**: Code generation, system building, infrastructure setup
- **13B - Artillery**: Heavy computation, batch processing, data bombardment
- **19D - Cavalry Scout**: Reconnaissance, codebase exploration, environment scanning

### 4.3 Support (SPT)

- **25B - IT Specialist**: System administration, deployment, DevOps
- **42A - HR Specialist**: Agent lifecycle management, swarm roster maintenance
- **88M - Transport**: Data transfer, API orchestration, message routing
- **92G - Logistics**: Resource allocation, dependency management, supply chain

### 4.4 Special Operations (SOF)

- **18B - SF Engineer**: Advanced system design, architecture, critical solutions
- **75th Ranger**: Rapid response, emergency fixes, crisis intervention
- **160th SOAR**: High-speed data operations, real-time processing

---

## 5. Swarm Coordination Architecture

### 5.1 Command & Control (C2)

**General's Command Post:**
- Receives mission from user/API
- Analyzes requirements and constraints
- Formulates strategy and objective tree
- Allocates resources (LLM quotas, compute, time)
- Issues orders to Colonels/Majors

**Battalion/Company Operations:**
- Break down objectives into executable tasks
- Assign tasks to appropriate MOS specialists
- Monitor progress via status reports
- Escalate blockers up chain of command
- Synchronize dependent operations

### 5.2 Task Delegation Pattern

```
User Request
    ↓
General (Mission Analysis)
    ↓
Major (Task Decomposition)
    ↓
Captain (Team Assignment)
    ↓
Sergeant (Execution Supervision)
    ↓
Specialist (Task Execution)
    ↓
Results → Roll-up through chain
```

### 5.3 Communication Protocols

1. **SITREP (Situation Report)**: Periodic status updates from subordinates
2. **FRAGO (Fragmentary Order)**: Mid-mission adjustments from superiors
3. **CASREP (Casualty Report)**: Agent failures/errors reported immediately
4. **AAR (After Action Review)**: Post-mission analysis and learning

### 5.4 Formation Types

| Formation | Size | Composition | Use Case |
|-----------|------|-------------|----------|
| Fire Team | 2-4 agents | 1 CPL + 2-3 SPC | Small focused task |
| Squad | 4-8 agents | 1 SGT + 1-2 CPL + 4-5 SPC | Module development |
| Platoon | 8-16 agents | 1 LT + 2-4 SGT + 8-12 SPC | Feature implementation |
| Company | 16-32 agents | 1 CPT + 3-4 LT + teams | Full application build |
| Battalion | 32+ agents | 1 COL + multiple CPTs + units | Complex system migration |

---

## 6. Technical Architecture

### 6.1 Agent Structure

```python
class RRAgent(BaseAgent):
    rank: str  # "CPT", "SGT", "SPC", etc.
    mos: str   # "35F", "12B", "11B", etc.
    unit: str  # "Alpha Company", "1st Platoon", etc.
    commander: Optional[str]  # Superior officer agent_id
    subordinates: List[str]  # Direct reports
    status: str  # "standby", "active", "engaged", "complete"
    mission: Optional[Mission]  # Current assignment

    def receive_order(self, order: Order) -> None
    def execute_mission(self) -> Result
    def report_sitrep(self) -> SitRep
    def delegate_task(self, task: Task, subordinate: str) -> None
```

### 6.2 Swarm Orchestrator

```python
class SwarmOrchestrator:
    def spawn_swarm(self, mission: Mission) -> Swarm
    def assign_general(self, swarm_id: str) -> RRAgent
    def build_unit(self, size: int, specialties: List[str]) -> Unit
    def monitor_operations(self, swarm_id: str) -> SwarmStatus
    def debrief_and_disband(self, swarm_id: str) -> AAR
```

### 6.3 Mission Object

```python
class Mission:
    id: str
    objective: str  # Natural language goal
    constraints: Dict  # Time, cost, quality requirements
    resources: Dict  # Available LLM calls, API keys, compute
    priority: str  # "routine", "priority", "urgent"
    tasks: List[Task]  # Decomposed work units
    status: str
    timeline: Dict
```

### 6.4 Database Schema

**Tables:**
- `swarms`: Swarm instances with General assignment
- `agents`: Individual agent records with rank/MOS
- `units`: Organizational structure (companies, platoons, squads)
- `missions`: Mission details and status
- `tasks`: Atomic work units assigned to agents
- `communications`: SITREP, FRAGO, CASREP logs
- `aars`: After Action Reviews for learning

---

## 7. Operational Workflows

### 7.1 Swarm Genesis

1. User submits mission via API
2. System spawns General agent
3. General analyzes mission and requests unit composition
4. Orchestrator builds requested units with appropriate MOS mix
5. General issues initial orders
6. Swarm activates and begins execution

### 7.2 Task Execution Cycle

1. Officer receives objective from superior
2. Officer decomposes into tasks
3. Officer assigns tasks to subordinates by MOS match
4. NCOs/Specialists execute tasks using LLM clients
5. Results reported up chain
6. Officer aggregates and reports to superior
7. Repeat until mission complete

### 7.3 Error Handling & Resilience

- **Agent Failure**: Sergeant reassigns task to backup specialist
- **Officer Failure**: NCO assumes temporary command, reports up
- **Resource Exhaustion**: General requests additional resources or revises strategy
- **Blocker**: Escalation up chain until rank with authority resolves

### 7.4 Swarm Termination

1. General declares mission complete
2. All agents submit final SITREPs
3. System conducts AAR (lessons learned)
4. Swarm disbanded, agents return to reserve pool
5. Results delivered to user

---

## 8. API Specifications

### 8.1 Core Endpoints

```
POST /rr/swarm/create
    - Input: Mission specification
    - Output: Swarm ID, General assignment

GET /rr/swarm/{swarm_id}/status
    - Output: Current swarm status, active agents, progress

POST /rr/swarm/{swarm_id}/frago
    - Input: Mission adjustment
    - Output: Updated orders issued

GET /rr/swarm/{swarm_id}/sitrep
    - Output: Aggregated status reports from all units

POST /rr/swarm/{swarm_id}/disband
    - Output: AAR report

GET /rr/agents/roster
    - Output: Available agents by rank/MOS

POST /rr/unit/create
    - Input: Unit type, size, MOS requirements
    - Output: Unit ID, assigned agents
```

### 8.2 Agent Communication API

```
POST /rr/agent/{agent_id}/order
    - Input: Order from superior

GET /rr/agent/{agent_id}/status

POST /rr/agent/{agent_id}/report
    - Input: SITREP/CASREP data

POST /rr/agent/{agent_id}/delegate
    - Input: Task, subordinate_id
```

---

## 9. LLM Integration Strategy

### 9.1 Rank-Based LLM Assignment

| Rank Level | Primary LLM | Rationale |
|------------|-------------|-----------|
| General/Colonel | Claude Opus 4.5 | Strategic reasoning, complex planning |
| Major/Captain | GPT-4 / Claude Sonnet | Tactical decomposition, coordination |
| Lieutenant/SGM | Gemini Pro / Mixtral | Supervision, quality control |
| Sergeant/Corporal | GPT-4 Mini / Llama | Task execution guidance |
| Specialist/Private | Groq Mixtral / DeepSeek | Fast, cost-effective execution |

### 9.2 Multi-LLM Consensus for Critical Decisions

- General uses 3-LLM consensus (Claude + GPT-4 + Gemini) for strategy
- Captains use 2-LLM validation for complex tasks
- Specialists use single LLM for routine execution

---

## 10. Security & Authentication

### 10.1 Agent Authentication

- Each agent has JWT token with rank/MOS claims
- Chain of command validated via signatures
- Only superior officers can issue orders to subordinates

### 10.2 Mission Access Control

- User credentials determine max swarm size/rank
- Mission sensitivity levels: UNCLASS, CONFIDENTIAL, SECRET
- Agents inherit clearance level from mission

### 10.3 Self-Preservation Integration

- Counterintelligence agents (35L) run continuous threat detection
- CASREP automatically triggered on threat detection
- Swarm can self-quarantine compromised agents

---

## 11. Performance Requirements

### 11.1 Scalability

- Support swarms from 1 to 100+ agents
- Sub-second C2 latency for orders/reports
- Handle 10+ concurrent swarms

### 11.2 Efficiency

- Minimize redundant LLM calls through task deduplication
- Parallel execution where dependencies allow
- Resource pooling for shared infrastructure tasks

### 11.3 Reliability

- 99% task completion rate for well-defined missions
- Graceful degradation under resource constraints
- Automatic retries with exponential backoff

---

## 12. User Interface

### 12.1 Web Dashboard

- **Command Center**: View all active swarms
- **Unit Roster**: Browse available agents by rank/MOS
- **Mission Control**: Create, monitor, adjust missions
- **AAR Archive**: Review past operations and learnings

### 12.2 CLI Interface

```bash
# Create swarm
rr deploy --mission "Build REST API for users" --priority urgent

# Monitor swarm
rr status swarm-12345

# Issue FRAGO
rr frago swarm-12345 "Add authentication to all endpoints"

# Disband
rr disband swarm-12345
```

---

## 13. Integration with Existing AgentX Foundry

### 13.1 Leverage Existing Components

- `BaseAgent` → extends to `RRAgent`
- `llm_integrations/clients.py` → used by all agents
- `recursion_engine.py` → used by Generals for iterative refinement
- `security.py` → enforces authentication and clearance
- `self_preservation.py` → integrates with 35L counterintel agents

### 13.2 New Components to Build

- `rr/swarm_orchestrator.py`: Core swarm management
- `rr/agent_factory.py`: Spawn agents with rank/MOS
- `rr/mission_planner.py`: General's strategic decomposition engine
- `rr/c2_router.py`: Route orders/reports through chain of command
- `rr/aar_engine.py`: Post-mission analysis and learning

---

## 14. Development Phases

### Phase 1: Foundation (Weeks 1-2)
- Implement RRAgent base class
- Create agent factory with rank/MOS system
- Build basic swarm orchestrator
- Simple mission → single agent execution

### Phase 2: Hierarchy (Weeks 3-4)
- Implement chain of command routing
- Add task delegation from officers to subordinates
- SITREP/FRAGO communication system
- Fire team (2-4 agent) coordination

### Phase 3: Intelligence (Weeks 5-6)
- General's mission planning engine
- Task decomposition algorithms
- Multi-LLM consensus integration
- Platoon-level (8-16 agent) operations

### Phase 4: Scale (Weeks 7-8)
- Company and battalion support
- Parallel execution optimization
- Resource management and quotas
- Full web dashboard

### Phase 5: Learning (Weeks 9-10)
- AAR analysis engine
- Pattern recognition from past missions
- Agent performance scoring
- Automated unit composition recommendations

---

## 15. Success Metrics

- **Mission Completion Rate**: >95% for well-defined objectives
- **Coordination Efficiency**: <10% overhead from C2 vs direct execution
- **User Satisfaction**: NPS >50 from beta testers
- **Cost Efficiency**: <20% LLM cost vs naive parallel execution
- **Swarm Formation Time**: <30s to spawn and activate 16-agent platoon

---

## 16. Future Enhancements

- **Promotions System**: Agents gain rank based on performance
- **Cross-Training**: Agents learn secondary MOS
- **Joint Operations**: Multiple swarms coordinate on mega-missions
- **Reserve Call-Up**: Persistent agent pool with memory of past missions
- **War Games**: Swarms compete/cooperate in simulated environments
- **NATO Integration**: Multi-organization swarm federations

---

## 17. Glossary

- **AAR**: After Action Review - post-mission analysis
- **C2**: Command & Control
- **CASREP**: Casualty Report - agent failure notification
- **FRAGO**: Fragmentary Order - mid-mission adjustment
- **MOS**: Military Occupational Specialty
- **NCO**: Non-Commissioned Officer
- **RR**: Rational Reserve
- **SITREP**: Situation Report - status update
- **SOF**: Special Operations Forces
- **swaRRm**: Colloquial name for Rational Reserve swarm

---

**Document Control:**
- **Author**: AgentX Foundry Team
- **Classification**: UNCLASSIFIED
- **Distribution**: Public (GitHub)
- **Review Cycle**: Quarterly or as needed

---

*"Many minds, one mission."* - Rational Reserve Motto
