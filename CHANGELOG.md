# Changelog

All notable changes to the Rational Reserve (AgentX Foundry) project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Database persistence layer
- Advanced C2 communication workflows
- Parallel task execution (async)
- Multi-swarm joint operations

## [0.2.1] - 2025-12-24

### Security
- **Dependency Updates**: Updated legacy package dependencies to address security vulnerabilities
  - Updated `legacy/package-lock.json` with 1733 additions, 513 deletions
  - Total changes: 2241 lines affected

- **ECDSA Risk Documentation**: Documented known elliptic curve cryptography (ECDSA) risks
  - Updated `requirements.txt` with security considerations
  - Added documentation for key management best practices

### Changed
- **Python Dependencies**: Updated 5 dependencies in `requirements.txt`
  - Enhanced security posture across the project
  - Ensured compatibility with latest security patches

---

## [0.2.0] - 2025-12-24

### Added - Phase 2-3: Autonomous Operations

#### General Cognitive Framework (`rr/general_mind.py`)
- **Recursive/Iterative Thinking**: Circular reasoning patterns for continuous refinement
  - `recursive_thinking_prompt()`: Generates prompts for cyclical analysis
  - Analyze → Decompose → Plan → Predict → Execute → Repeat
  - All strategies contain seeds of their own evolution

- **Reactive Complex**: Instant pivot capability
  - `reactive_complex_prompt()`: Generates prompts for adaptive pivoting
  - SENSE → EVALUATE → DECIDE → ACT cycle
  - Pivot decisions: PARRY (defend), LUNGE (exploit), DISENGAGE (redesign), HOLD (maintain)
  - Millisecond-level adaptation to environmental changes

- **Predictive Engine**: Event horizon analysis
  - `predictive_engine_prompt()`: Generates prompts for foresight analysis
  - Projects future states (T+1, T+2, T+3)
  - Calculates optimal positioning based on predicted trajectories
  - Telegraphs moves with quantum precision

- **Unified Strategic Framework**:
  - `unified_strategic_prompt()`: Combines all three frameworks
  - Integrated cognitive system for General-level decision making
  - Hardcoded into General's task execution logic

#### Autonomous Mission Planning (`rr/mission_planner.py`)
- **MissionPlanner Class**: Converts objectives into structured task plans
  - `plan_mission()`: Autonomous mission decomposition
  - Uses General Mind framework for strategic planning
  - Generates task graphs with dependencies and MOS assignments
  - JSON parsing with text fallback for robust task extraction

- **Adaptive Planning**:
  - `adapt_plan()`: Reactive re-planning on detected changes
  - Merges current progress with new requirements
  - Preserves completed work while adjusting strategy

- **Predictive Execution**:
  - `predict_execution()`: Forecasts future execution states
  - Calculates progress velocity and acceleration
  - Identifies potential blockers before they manifest
  - Recommends optimal positioning

- **Task Parsing**:
  - JSON-based structured task extraction
  - Text-based fallback parsing
  - Automatic task metadata enrichment

#### Swarm Orchestrator Enhancements (`rr/swarm_orchestrator.py`)
- **Autonomous Mode Toggle**:
  - `enable_autonomous_planning` parameter (default: True)
  - Automatic fallback to Phase 1 legacy mode if LLM unavailable
  - Runtime mode switching capability

- **Mission Planning Integration**:
  - Initializes `MissionPlanner` with Anthropic LLM for strategic planning
  - Automatic swarm composition analysis for planning context

- **Enhanced Mission Execution**:
  - **Autonomous Mode**: General plans, decomposes, and delegates
  - **Legacy Mode**: General executes directly (Phase 1 compatibility)
  - Mode indicated in execution results

- **Task Delegation System**:
  - `_get_swarm_composition()`: Analyzes available agents by rank/MOS
  - `_assign_tasks_to_agents()`: Matches tasks to agents by specialty
  - Intelligent fallback: General handles unmatched tasks

- **Dependency-Aware Execution**:
  - `_execute_task_graph()`: Respects task dependencies
  - Sequential execution with dependency validation
  - Deadlock detection and recovery
  - Per-task result tracking

#### RRAgent Integration (`rr/rr_agent.py`)
- **General Cognitive Framework Integration**:
  - Generals automatically use `GeneralMind.unified_strategic_prompt()`
  - Standard agents continue using MOS-based prompts
  - Context-aware prompt generation with mission metadata

### Technical Improvements

#### Strategic Decision Making
- Generals now think recursively, reactively, and predictively
- All strategic decisions encoded in prompts (no hidden logic)
- Reproducible and auditable cognitive processes

#### Autonomous Operation
- Swarms can operate without constant human direction
- General analyzes objectives and creates execution plans
- Task assignment based on agent capabilities
- Automatic adaptation to changing conditions

#### Execution Reliability
- Dependency management prevents task ordering issues
- Deadlock detection prevents infinite loops
- Comprehensive error handling at every level
- Detailed logging of all strategic decisions

### Statistics
- **New Files**: 2 (general_mind.py, mission_planner.py)
- **Modified Files**: 2 (rr_agent.py, swarm_orchestrator.py)
- **New Methods**: 15+
- **Lines Added**: ~800
- **Cognitive Frameworks**: 3 (Recursive, Reactive, Predictive)

### Notes
- Phase 2-3 achieves autonomous mission execution
- General's cognitive framework is hardcoded (circular, reactive, prescient)
- Task decomposition and delegation fully operational
- System can now work independently toward high-level goals
- Foundation ready for BCI integration
- Target deadline: 2025-12-31

---

## [0.1.0] - 2025-12-24

### Added - Phase 1 Foundation

#### Core Data Models (`rr/models.py`)
- **Enumerations:**
  - `RankEnum`: 12 military ranks (PVT, PFC, SPC, CPL, SGT, MSG, SGM, LT, CPT, MAJ, COL, GEN)
  - `MOSEnum`: 16 Military Occupational Specialties across 4 categories:
    - Intelligence: 35F (Analyst), 35L (Counterintel), 35N (SIGINT)
    - Operations: 11B (Infantry), 12B (Engineer), 13B (Artillery), 19D (Scout)
    - Support: 25B (IT), 42A (HR), 88M (Transport), 92G (Logistics)
    - Special Ops: 18B (SF Engineer), 75R (Ranger), 160 (SOAR)
  - `StatusEnum`: Agent/mission status states
  - `PriorityEnum`: Mission priority levels

- **Data Structures:**
  - `Task`: Atomic work unit with dependencies, assignment tracking
  - `Mission`: High-level objective with task decomposition
  - `Order`: Command from superior to subordinate
  - `SitRep`: Situation report from subordinate to superior
  - `CasRep`: Casualty report for agent failures
  - `AAR`: After Action Review for post-mission analysis
  - `Unit`: Organizational structure (squad, platoon, company)
  - `Swarm`: Complete swarm instance with agents and units
  - `AgentRecord`: Persistent agent record for database

#### RRAgent Implementation (`rr/rr_agent.py`)
- **Core Class:**
  - Extends `BaseAgent` with military rank and MOS
  - Chain of command validation and enforcement
  - Rank-based LLM selection algorithm:
    - General/Colonel: Anthropic (Claude Opus/Sonnet)
    - Major/Captain: OpenAI (GPT-4)
    - Lieutenant/SGM: Google (Gemini)
    - Sergeant/Corporal: Groq (Mixtral)
    - Specialist/Private: DeepSeek

- **Capabilities:**
  - `receive_order()`: Accept and validate orders from superiors
  - `execute_task()`: Execute tasks with MOS-specific prompting
  - `delegate_task()`: Delegate tasks to subordinates
  - `report_sitrep()`: Generate situation reports
  - `report_casrep()`: Generate casualty reports on failure
  - `can_command()`: Validate command authority by rank

- **MOS-Specific Task Prompting:**
  - Each MOS has specialized guidance for task execution
  - Intelligence MOS: Analysis, threat detection, pattern recognition
  - Operations MOS: Execution, construction, reconnaissance
  - Support MOS: Infrastructure, resource management
  - Special Ops MOS: Advanced solutions, rapid response

#### Agent Factory (`rr/agent_factory.py`)
- **Individual Agent Spawning:**
  - `spawn_agent()`: Create agent with specific rank/MOS
  - `spawn_general()`: Spawn swarm commander
  - `spawn_officer()`: Spawn COL/MAJ/CPT/LT
  - `spawn_nco()`: Spawn SGM/MSG/SGT/CPL
  - `spawn_specialist()`: Spawn SPC with specific MOS

- **Formation Building:**
  - `spawn_fire_team()`: 1 leader (CPL/SGT) + 3 specialists (4 total)
  - `spawn_squad()`: 1 SGT + 2 fire teams (7 total)
  - `spawn_platoon()`: 1 LT + 3 squads (25+ total)
  - Automatic chain of command setup
  - Auto-generated agent naming

- **Management:**
  - Active agent tracking and roster maintenance
  - Agent lifecycle management (spawn/deactivate)
  - Statistics and filtering by rank/MOS/status
  - Agent counter for unique naming

#### Swarm Orchestrator (`rr/swarm_orchestrator.py`)
- **Mission Management:**
  - `create_mission()`: Create mission with objective, priority, constraints
  - `get_mission()`: Retrieve mission by ID
  - Mission status tracking and lifecycle

- **Swarm Lifecycle:**
  - `create_swarm()`: Spawn swarm with General and units for mission
  - `activate_swarm()`: Activate swarm for mission execution
  - `execute_mission()`: Execute mission through General (Phase 1: direct execution)
  - `disband_swarm()`: Disband swarm and generate AAR
  - `get_swarm_status()`: Real-time swarm status reporting

- **Formation Construction:**
  - `_build_swarm_units()`: Build units based on size specification
  - Support for: single, fire_team, squad, platoon
  - Automatic MOS-based unit composition

- **Analytics:**
  - `get_active_swarms()`: List all active swarms
  - `get_stats()`: System-wide statistics
  - `_generate_aar()`: After Action Review generation

#### C2 Router (`rr/c2_router.py`)
- **Communication Routing:**
  - `route_order()`: Route orders down chain of command
  - `route_sitrep()`: Route situation reports up chain
  - `route_casrep()`: Route casualty reports with escalation
  - `broadcast_order()`: Broadcast to all subordinates

- **Chain of Command Validation:**
  - `_validate_chain_of_command()`: Verify superior can command subordinate
  - Direct subordinate checking
  - Rank-based override capability

- **Communication Logs:**
  - Order log with acknowledgment tracking
  - SITREP log with progress monitoring
  - CASREP log with failure tracking
  - `get_communication_stats()`: Communication statistics
  - `clear_logs()`: Log management and cleanup

- **Query Functions:**
  - `collect_sitreps()`: Collect all SITREPs for commander
  - `get_recent_sitreps()`: Get recent SITREPs from agent
  - `get_casreps()`: Get casualty reports with timestamp filtering
  - `get_order_history()`: Get order history for agent

#### FastAPI Integration (`rr/api_routes.py`)
- **Mission Endpoints:**
  - `POST /rr/mission/create`: Create mission and spawn swarm
  - `POST /rr/mission/quick`: Quick single-shot mission execution

- **Swarm Endpoints:**
  - `POST /rr/swarm/{id}/execute`: Activate and execute swarm mission
  - `GET /rr/swarm/{id}/status`: Get real-time swarm status
  - `POST /rr/swarm/{id}/disband`: Disband swarm and get AAR
  - `GET /rr/swarms`: List all active swarms

- **System Endpoints:**
  - `GET /rr/`: System information
  - `GET /rr/stats`: System statistics
  - `GET /rr/agents/roster`: Active agent roster

- **Request/Response Models:**
  - `CreateMissionRequest`: Mission creation parameters
  - `CreateSwarmResponse`: Swarm creation result
  - `ExecuteMissionResponse`: Mission execution result

#### Main Application Integration (`main.py`)
- **RR System Initialization:**
  - Initialize `AgentFactory` with LLM clients
  - Initialize `C2Router` with agent factory
  - Initialize `SwarmOrchestrator` with agent factory
  - Set orchestrator for API routes
  - Include RR router under `/rr` prefix

- **Integration Points:**
  - Shared LLM clients from `llm_integrations.clients`
  - JWT authentication via `utils.security.verify_token`
  - Logging via loguru
  - Coexistence with legacy predictive agent

#### Testing (`test_rr_phase1.py`)
- **Test Suite:**
  - Test 1: Agent creation (General, Specialist, Fire Team)
  - Test 2: Swarm creation and status
  - Test 3: Mission execution
  - Test 4: Swarm disbanding with AAR
  - Test 5: C2 communication routing

- **Coverage:**
  - Agent factory functionality
  - Swarm orchestrator lifecycle
  - C2 router communication
  - End-to-end mission execution

#### Documentation
- **Requirements Documentation:**
  - `REQUIREMENTS_RR.md`: Complete system requirements (17 sections)
  - Military hierarchy definitions
  - MOS specifications
  - Technical architecture
  - API specifications
  - Development phases
  - Success metrics

- **Completion Documentation:**
  - `RR_PHASE1_COMPLETE.md`: Phase 1 summary and guide
  - Architecture highlights
  - Usage examples (API and Python)
  - File structure
  - Next steps for Phase 2

- **Project Documentation:**
  - Updated `CLAUDE.md` with RR system overview
  - Development commands and patterns

### Technical Details

#### Architecture Patterns
- **Hierarchical Command Structure:**
  - Officers (strategic): GEN, COL, MAJ, CPT, LT
  - NCOs (tactical): SGM, MSG, SGT, CPL
  - Enlisted (execution): SPC, PFC, PVT

- **Communication Flow:**
  - Orders flow down: GEN → MAJ → CPT → LT → SGT → CPL → SPC
  - SITREPs flow up: SPC → CPL → SGT → LT → CPT → MAJ → GEN
  - CASREPs escalate to commander with immediate notification

- **Formation Hierarchy:**
  - Fire Team (4): 1 CPL + 3 SPC
  - Squad (7): 1 SGT + 2 fire teams
  - Platoon (25+): 1 LT + 3 squads
  - Company: 1 CPT + multiple platoons
  - Battalion: 1 COL + multiple companies

#### Phase 1 Limitations (By Design)
- No task decomposition (General executes directly)
- No parallel execution (sequential only)
- No database persistence (in-memory only)
- Simple AAR generation
- Limited C2 utilization (logged but not fully utilized)

#### Dependencies
- FastAPI for REST API
- Pydantic for data validation
- Loguru for logging
- Existing LLM integration layer
- Existing security/authentication layer

### Statistics
- **Lines of Code:** ~2,000
- **Files Created:** 7 core files + 3 documentation files
- **Data Models:** 11 Pydantic models
- **Ranks Supported:** 12
- **MOS Specialties:** 16
- **API Endpoints:** 8
- **Formation Types:** 4

### Notes
- Phase 1 provides foundation for autonomous swarm operations
- System is operational but simplified for testing
- All agents use in-memory state (no persistence)
- LLM selection is automatic based on rank
- Chain of command is enforced at all communication points

---

## Version History

### [0.1.0] - 2025-12-24
- Initial implementation of Rational Reserve Phase 1
- Foundation for autonomous agentic swarm system
- Military-structured hierarchy with 12 ranks and 16 MOS
- Basic swarm lifecycle management
- C2 communication framework
- FastAPI integration

---

## Roadmap

### Phase 2-3 (Target: January 1, 2026)
- Autonomous mission planning engine
- Task decomposition and delegation
- Parallel execution across swarm
- Self-organizing coordination
- General cognitive framework enhancements
- Database persistence

### Future Phases
- Learning and adaptation systems
- Performance-based promotions
- Cross-training and secondary MOS
- Multi-swarm joint operations
- BCI integration framework

---

**Motto:** "Many minds, one mission"

**Status:** Phase 1 Complete, Phase 2-3 In Progress
