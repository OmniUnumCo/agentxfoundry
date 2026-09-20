"""
Swarm Orchestrator - Manages RR swarm lifecycle
"""

from typing import Dict, Any, List, Optional
from loguru import logger
from datetime import datetime

from .models import (
    Mission, Swarm, Unit, StatusEnum, RankEnum, MOSEnum, RoleEnum,
    AAR, Task, PriorityEnum
)
from .agent_factory import AgentFactory
from .rr_agent import RRAgent
from .mission_planner import MissionPlanner
from .db_persistence import RRDatabase


class SwarmOrchestrator:
    """
    Orchestrates swarm creation, operation, and termination

    Manages the high-level swarm lifecycle and coordinates with
    the AgentFactory for agent management

    Phase 2-3: Includes autonomous mission planning and delegation
    """

    def __init__(self, agent_factory: AgentFactory, enable_autonomous_planning: bool = True, enable_db_persistence: bool = True):
        self.agent_factory = agent_factory
        self.active_swarms: Dict[str, Swarm] = {}
        self.missions: Dict[str, Mission] = {}
        self.units: Dict[str, Unit] = {}
        self.enable_autonomous_planning = enable_autonomous_planning
        self.enable_db_persistence = enable_db_persistence

        # Initialize database persistence
        self.db = None
        if enable_db_persistence:
            try:
                self.db = RRDatabase()
                logger.info("SwarmOrchestrator initialized with DATABASE PERSISTENCE enabled")
            except Exception as e:
                logger.warning(f"Database initialization failed: {e}, continuing without persistence")
                self.enable_db_persistence = False

        # Initialize mission planner if autonomous mode enabled
        self.mission_planner = None
        if enable_autonomous_planning:
            # Get LLM client for strategic planning (Orchestrator-level)
            if "anthropic" in agent_factory.llm_clients:
                self.mission_planner = MissionPlanner(agent_factory.llm_clients["anthropic"])
                logger.info("SwarmOrchestrator initialized with AUTONOMOUS PLANNING (Orchestrator) enabled")
            else:
                logger.warning("Anthropic LLM not available, autonomous planning disabled")
                self.enable_autonomous_planning = False
        else:
            logger.info("SwarmOrchestrator initialized in legacy mode (no autonomous planning)")

    def create_swarm(
        self,
        mission: Mission,
        swarm_size: str = "fire_team",
        primary_mos: MOSEnum = MOSEnum.INFANTRY,
        include_commander: bool = False,
        specialists_per_workstream: int = 1,
    ) -> Swarm:
        """
        Create a swarm with Jason hierarchy:

          [Commander (optional)] → Orchestrator → Director(s)
          Specialists attach as freelance advisors to Directors.

        Caps: max 1 Director + 1–2 Specialist advisors per workstream.
        No flat General→everyone fan-out.

        Args:
            mission: Mission object with objectives
            swarm_size: workstream count proxy (single|fire_team|squad|platoon)
            primary_mos: Primary MOS metadata for Directors/Specialists
            include_commander: If True, spawn optional Commander above Orchestrator
            specialists_per_workstream: 0–2 Specialist advisors per Director

        Returns:
            Created Swarm instance
        """
        logger.info(f"Creating hierarchical swarm for mission: {mission.objective}")

        if specialists_per_workstream < 0 or specialists_per_workstream > 2:
            raise ValueError("specialists_per_workstream must be 0–2")

        swarm = Swarm(
            name=f"Swarm-{mission.id[:8]}",
            mission_id=mission.id,
            status=StatusEnum.IDLE,
        )

        commander = None
        if include_commander:
            commander = self.agent_factory.spawn_commander(
                name=f"CMD-{swarm.id[:8]}"
            )
            commander.swarm_id = swarm.id
            commander.current_mission = mission.id
            swarm.commander_id = commander.agent_id
            swarm.agents.append(commander.agent_id)

        orchestrator = self.agent_factory.spawn_orchestrator(
            name=f"ORCH-{swarm.id[:8]}",
            commander_id=commander.agent_id if commander else None,
        )
        orchestrator.swarm_id = swarm.id
        orchestrator.current_mission = mission.id
        swarm.orchestrator_id = orchestrator.agent_id
        swarm.general_id = orchestrator.agent_id  # legacy alias
        swarm.agents.append(orchestrator.agent_id)
        if commander:
            commander.add_subordinate(orchestrator.agent_id)

        workstream_count = {
            "single": 1,
            "fire_team": 1,
            "squad": 2,
            "platoon": 3,
            "company": 4,
        }.get(swarm_size, 1)

        for i in range(workstream_count):
            ws = self.agent_factory.spawn_workstream(
                orchestrator_id=orchestrator.agent_id,
                primary_mos=primary_mos,
                specialist_count=specialists_per_workstream,
                director_name=f"DIR-{swarm.id[:8]}-{i+1}",
            )
            for agent in ws["agents"]:
                agent.swarm_id = swarm.id
                agent.current_mission = mission.id
                swarm.agents.append(agent.agent_id)

        self.active_swarms[swarm.id] = swarm
        self.missions[mission.id] = mission
        mission.assigned_swarm = swarm.id

        logger.info(
            f"Swarm created: {swarm.name} with {len(swarm.agents)} agents "
            f"(Orchestrator: {orchestrator.name}, workstreams={workstream_count})"
        )
        return swarm

    def activate_swarm(self, swarm_id: str) -> bool:
        """
        Activate swarm and begin mission execution

        Args:
            swarm_id: Swarm ID to activate

        Returns:
            True if activation successful
        """
        if swarm_id not in self.active_swarms:
            logger.error(f"Swarm {swarm_id} not found")
            return False

        swarm = self.active_swarms[swarm_id]
        mission = self.missions.get(swarm.mission_id)

        if not mission:
            logger.error(f"Mission {swarm.mission_id} not found")
            return False

        # Update status (lifecycle: IDLE → for swarm we use EXECUTING when active)
        swarm.status = StatusEnum.EXECUTING
        swarm.activated_at = datetime.utcnow()
        mission.status = StatusEnum.EXECUTING
        mission.started_at = datetime.utcnow()

        # Activate Orchestrator (and optional Commander)
        orch_id = swarm.orchestrator_id or swarm.general_id
        orchestrator = self.agent_factory.get_agent(orch_id) if orch_id else None
        if orchestrator:
            orchestrator.status = StatusEnum.EXECUTING
        if swarm.commander_id:
            commander = self.agent_factory.get_agent(swarm.commander_id)
            if commander:
                commander.status = StatusEnum.EXECUTING

        logger.info(f"Swarm {swarm.name} activated for mission: {mission.objective}")
        return True

    def execute_mission(self, swarm_id: str) -> Dict[str, Any]:
        """
        Execute mission through swarm

        Phase 2-3: Autonomous mission planning and delegation
        - General analyzes mission and decomposes into tasks
        - Tasks delegated to appropriate agents by MOS/rank
        - Parallel execution where dependencies allow
        - Adaptive re-planning on failures

        Args:
            swarm_id: Swarm ID

        Returns:
            Execution result
        """
        if swarm_id not in self.active_swarms:
            raise ValueError(f"Swarm {swarm_id} not found")

        swarm = self.active_swarms[swarm_id]
        mission = self.missions[swarm.mission_id]

        logger.info(f"Executing mission via swarm {swarm.name}")

        # Get Orchestrator (legacy general_id still supported)
        orch_id = swarm.orchestrator_id or swarm.general_id
        general = self.agent_factory.get_agent(orch_id)  # variable name kept for minimal churn
        if not general:
            raise ValueError(f"Orchestrator not found for swarm {swarm_id}")

        try:
            if self.enable_autonomous_planning and self.mission_planner:
                # PHASE 2-3: AUTONOMOUS PLANNING AND DELEGATION
                logger.info("Using autonomous mission planning")

                # Get swarm composition
                swarm_composition = self._get_swarm_composition(swarm)

                # General plans the mission autonomously
                planned_tasks = self.mission_planner.plan_mission(
                    mission=mission,
                    swarm_composition=swarm_composition
                )

                mission.tasks = planned_tasks
                logger.info(f"Mission decomposed into {len(planned_tasks)} tasks")

                # Assign tasks to agents based on MOS/rank requirements
                self._assign_tasks_to_agents(planned_tasks, swarm)

                # Execute tasks with dependency management
                results = self._execute_task_graph(planned_tasks, swarm)

                # Aggregate results
                all_successful = all(r.get("success", False) for r in results)

                if all_successful:
                    mission.status = StatusEnum.REPORTING
                    mission.completed_at = datetime.utcnow()
                    mission.result = {"tasks_executed": len(results), "results": results}
                    swarm.status = StatusEnum.REPORTING

                    logger.info(f"Mission completed successfully: {len(results)} tasks executed")

                    return {
                        "success": True,
                        "mission_id": mission.id,
                        "swarm_id": swarm.id,
                        "result": mission.result,
                        "mode": "autonomous",
                    }
                else:
                    failed_tasks = [r for r in results if not r.get("success", False)]
                    logger.error(f"Mission partially failed: {len(failed_tasks)} tasks failed")

                    mission.status = StatusEnum.FAILED
                    swarm.status = StatusEnum.FAILED

                    return {
                        "success": False,
                        "mission_id": mission.id,
                        "swarm_id": swarm.id,
                        "error": f"{len(failed_tasks)} tasks failed",
                        "failed_tasks": failed_tasks,
                        "mode": "autonomous",
                    }

            else:
                # PHASE 1: LEGACY MODE - Orchestrator executes directly
                logger.info("Using legacy direct execution mode")

                task = Task(
                    description=mission.objective,
                    assigned_to=general.agent_id,
                )
                mission.tasks.append(task)

                result = general.execute_task(task)

                mission.status = StatusEnum.REPORTING
                mission.completed_at = datetime.utcnow()
                mission.result = result
                swarm.status = StatusEnum.REPORTING

                logger.info(f"Mission completed successfully: {mission.objective}")

                return {
                    "success": True,
                    "mission_id": mission.id,
                    "swarm_id": swarm.id,
                    "result": result,
                    "mode": "legacy",
                }

        except Exception as e:
            logger.error(f"Mission execution failed: {str(e)}")
            mission.status = StatusEnum.FAILED
            swarm.status = StatusEnum.FAILED

            return {
                "success": False,
                "mission_id": mission.id,
                "swarm_id": swarm.id,
                "error": str(e),
            }

    def get_swarm_status(self, swarm_id: str) -> Dict[str, Any]:
        """
        Get current swarm status

        Args:
            swarm_id: Swarm ID

        Returns:
            Status information
        """
        if swarm_id not in self.active_swarms:
            return {"error": "Swarm not found"}

        swarm = self.active_swarms[swarm_id]
        mission = self.missions.get(swarm.mission_id)

        # Collect agent status
        agents_status = []
        for agent_id in swarm.agents:
            agent = self.agent_factory.get_agent(agent_id)
            if agent:
                agents_status.append({
                    "id": agent.agent_id,
                    "name": agent.name,
                    "role": agent.role.value,
                    "rank": agent.rank.value if agent.rank else None,
                    "mos": agent.mos.value if agent.mos else None,
                    "status": agent.status.value,
                })

        return {
            "swarm_id": swarm.id,
            "name": swarm.name,
            "status": swarm.status.value,
            "mission": {
                "id": mission.id if mission else None,
                "objective": mission.objective if mission else None,
                "status": mission.status.value if mission else None,
            },
            "agent_count": len(swarm.agents),
            "agents": agents_status,
            "created_at": swarm.created_at.isoformat(),
            "activated_at": swarm.activated_at.isoformat() if swarm.activated_at else None,
        }

    def disband_swarm(self, swarm_id: str) -> AAR:
        """
        Disband swarm and generate After Action Review

        Args:
            swarm_id: Swarm ID

        Returns:
            AAR report
        """
        if swarm_id not in self.active_swarms:
            raise ValueError(f"Swarm {swarm_id} not found")

        swarm = self.active_swarms[swarm_id]
        mission = self.missions.get(swarm.mission_id)

        logger.info(f"Disbanding swarm: {swarm.name}")

        # Generate AAR
        aar = self._generate_aar(swarm, mission)

        # Deactivate all agents
        for agent_id in swarm.agents:
            self.agent_factory.deactivate_agent(agent_id)

        # Mark swarm as disbanded
        swarm.status = StatusEnum.IDLE
        swarm.disbanded_at = datetime.utcnow()

        # Remove from active swarms
        del self.active_swarms[swarm_id]

        logger.info(f"Swarm {swarm.name} disbanded")

        return aar

    def _build_swarm_units(
        self,
        swarm_size: str,
        primary_mos: MOSEnum,
        commander_id: str,
        swarm_id: str,
    ) -> List[RRAgent]:
        """
        Build swarm units based on size specification

        Args:
            swarm_size: Size category
            primary_mos: Primary MOS
            commander_id: Commander agent ID
            swarm_id: Swarm ID

        Returns:
            List of spawned agents
        """
        # Legacy formation helpers — prefer create_swarm workstream path.
        # Directors own workstreams; Specialists are advisors (not line CoC).
        if swarm_size == "single":
            director = self.agent_factory.spawn_director(
                mos=primary_mos,
                commander_id=commander_id,
            )
            return [director]
        elif swarm_size == "squad":
            return self.agent_factory.spawn_squad(
                primary_mos=primary_mos,
                commander_id=commander_id,
            )
        elif swarm_size == "platoon":
            return self.agent_factory.spawn_platoon(
                primary_mos=primary_mos,
                commander_id=commander_id,
            )
        else:
            return self.agent_factory.spawn_fire_team(
                mos=primary_mos,
                team_size=4,
                commander_id=commander_id,
            )

    def _generate_aar(self, swarm: Swarm, mission: Optional[Mission]) -> AAR:
        """
        Generate After Action Review

        Args:
            swarm: Swarm instance
            mission: Mission instance

        Returns:
            AAR report
        """
        success = bool(mission and mission.completed_at and mission.status != StatusEnum.FAILED)

        aar = AAR(
            mission_id=mission.id if mission else "unknown",
            swarm_id=swarm.id,
            success=success,
            objectives_met=[mission.objective] if success else [],
            objectives_missed=[] if success else [mission.objective] if mission else [],
            lessons_learned=[
                "Phase 1 implementation: General-only execution",
                "Future: Implement task decomposition and delegation",
            ],
            agent_performance={
                agent_id: {"tasks_completed": 1}
                for agent_id in swarm.agents
            },
            resource_usage={
                "total_agents": len(swarm.agents),
                "llm_calls": "not tracked in Phase 1",
            },
            timeline={
                "created": swarm.created_at,
                "activated": swarm.activated_at or swarm.created_at,
                "completed": swarm.disbanded_at or datetime.utcnow(),
            },
            recommendations=[
                "Implement task decomposition in Phase 2",
                "Add multi-agent coordination",
                "Track LLM usage and costs",
            ],
        )

        return aar

    def get_active_swarms(self) -> List[Dict[str, Any]]:
        """Get all active swarms with basic info"""
        return [
            {
                "id": swarm.id,
                "name": swarm.name,
                "status": swarm.status.value,
                "agent_count": len(swarm.agents),
                "mission_id": swarm.mission_id,
            }
            for swarm in self.active_swarms.values()
        ]

    def get_mission(self, mission_id: str) -> Optional[Mission]:
        """Get mission by ID"""
        return self.missions.get(mission_id)

    def create_mission(
        self,
        objective: str,
        description: Optional[str] = None,
        priority: PriorityEnum = PriorityEnum.ROUTINE,
        constraints: Optional[Dict[str, Any]] = None,
    ) -> Mission:
        """
        Create a new mission

        Args:
            objective: Mission objective (natural language)
            description: Detailed description
            priority: Mission priority
            constraints: Mission constraints

        Returns:
            Created Mission instance
        """
        mission = Mission(
            objective=objective,
            description=description,
            priority=priority,
            constraints=constraints or {},
        )

        self.missions[mission.id] = mission
        logger.info(f"Mission created: {mission.objective} (ID: {mission.id})")

        return mission

    def get_stats(self) -> Dict[str, Any]:
        """Get orchestrator statistics"""
        return {
            "active_swarms": len(self.active_swarms),
            "total_missions": len(self.missions),
            "completed_missions": len(
                [m for m in self.missions.values() if m.status in (StatusEnum.REPORTING, StatusEnum.IDLE) and m.completed_at]
            ),
            "failed_missions": len(
                [m for m in self.missions.values() if m.status == StatusEnum.FAILED]
            ),
            "agent_factory_stats": self.agent_factory.get_stats(),
            "autonomous_planning_enabled": self.enable_autonomous_planning,
        }

    # Phase 2-3 Autonomous Execution Methods

    def _get_swarm_composition(self, swarm: Swarm) -> Dict[str, List[str]]:
        """
        Get swarm composition for mission planning

        Args:
            swarm: Swarm instance

        Returns:
            Dict with agents organized by rank and MOS
        """
        composition = {
            "by_role": {},
            "by_rank": {},
            "by_mos": {},
        }

        for agent_id in swarm.agents:
            agent = self.agent_factory.get_agent(agent_id)
            if agent:
                role_key = agent.role.value
                composition["by_role"].setdefault(role_key, []).append(agent_id)
                if agent.rank:
                    composition["by_rank"].setdefault(agent.rank.value, []).append(agent_id)
                if agent.mos:
                    composition["by_mos"].setdefault(agent.mos.value, []).append(agent_id)

        return composition

    def _assign_tasks_to_agents(self, tasks: List[Task], swarm: Swarm):
        """
        Assign tasks to appropriate agents based on MOS/rank requirements

        Args:
            tasks: List of tasks to assign
            swarm: Swarm instance with available agents
        """
        orch_id = swarm.orchestrator_id or swarm.general_id
        for task in tasks:
            required_role = (task.metadata.get("assigned_role") or "").lower()
            required_mos = task.metadata.get("assigned_mos")
            required_rank = task.metadata.get("assigned_rank")

            suitable_agent = None
            # Prefer Directors for owned work; Specialists only if role=specialist (advisory)
            candidates = []
            for agent_id in swarm.agents:
                agent = self.agent_factory.get_agent(agent_id)
                if not agent:
                    continue
                if agent.role in (RoleEnum.COMMANDER, RoleEnum.ORCHESTRATOR):
                    continue  # plan/delegate, do not execute workstream tasks
                if agent.agent_id == orch_id or agent.agent_id == swarm.commander_id:
                    continue
                candidates.append(agent)

            # Role preference
            if required_role == "specialist":
                pool = [a for a in candidates if a.role == RoleEnum.SPECIALIST]
            else:
                # Default: Directors own execution
                pool = [a for a in candidates if a.role == RoleEnum.DIRECTOR] or candidates

            for agent in pool:
                mos_match = (not required_mos) or (agent.mos and agent.mos.value == required_mos)
                rank_match = (not required_rank) or (agent.rank and agent.rank.value == required_rank)
                agent_available = agent.status in (StatusEnum.IDLE, StatusEnum.BRIEFED)
                if mos_match and rank_match and agent_available:
                    suitable_agent = agent
                    break

            if suitable_agent:
                task.assigned_to = suitable_agent.agent_id
                logger.info(
                    f"Task '{task.description[:50]}...' assigned to {suitable_agent.name} "
                    f"[role={suitable_agent.role.value}]"
                )
            else:
                task.assigned_to = orch_id
                logger.warning(
                    f"No suitable Director for task '{task.description[:50]}...', "
                    f"assigning to Orchestrator"
                )

    def _execute_task_graph(self, tasks: List[Task], swarm: Swarm) -> List[Dict[str, Any]]:
        """
        Execute tasks with dependency management

        Phase 2-3: Respects task dependencies and executes in correct order
        Future: Parallel execution where dependencies allow

        Args:
            tasks: List of tasks with dependencies
            swarm: Swarm instance

        Returns:
            List of execution results
        """
        results = []
        completed_task_ids = set()

        # Simple sequential execution respecting dependencies
        # Future enhancement: Parallel execution with dependency graph
        max_iterations = len(tasks) * 2  # Prevent infinite loops
        iteration = 0

        while len(completed_task_ids) < len(tasks) and iteration < max_iterations:
            iteration += 1
            progress_made = False

            for task in tasks:
                # Skip if already completed
                if task.id in completed_task_ids:
                    continue

                # Check if dependencies are met
                dependencies_met = all(
                    dep_id in completed_task_ids
                    for dep_id in task.dependencies
                )

                if not dependencies_met:
                    continue  # Wait for dependencies

                # Execute task
                agent = self.agent_factory.get_agent(task.assigned_to)
                if not agent:
                    logger.error(f"Agent {task.assigned_to} not found for task {task.id}")
                    results.append({
                        "task_id": task.id,
                        "success": False,
                        "error": "Agent not found"
                    })
                    completed_task_ids.add(task.id)
                    progress_made = True
                    continue

                try:
                    logger.info(
                        f"Executing task: '{task.description[:50]}...' "
                        f"via {agent.name}"
                    )
                    result = agent.execute_task(task)

                    results.append({
                        "task_id": task.id,
                        "agent_id": agent.agent_id,
                        "success": True,
                        "result": result
                    })
                    completed_task_ids.add(task.id)
                    progress_made = True

                except Exception as e:
                    logger.error(f"Task execution failed: {str(e)}")
                    results.append({
                        "task_id": task.id,
                        "agent_id": agent.agent_id,
                        "success": False,
                        "error": str(e)
                    })
                    completed_task_ids.add(task.id)
                    progress_made = True

            # If no progress made in this iteration, we're stuck
            if not progress_made:
                logger.error("Dependency deadlock or all remaining tasks blocked")
                # Mark remaining tasks as failed
                for task in tasks:
                    if task.id not in completed_task_ids:
                        results.append({
                            "task_id": task.id,
                            "success": False,
                            "error": "Dependency deadlock or blocked"
                        })
                break

        return results
