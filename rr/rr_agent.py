"""
RRAgent - Rational Reserve Agent with RoleEnum chain of command

Line authority: Commander → Orchestrator → Director
Specialists: freelance advisors (consult; do not command Directors)
Lifecycle: Idle → Briefed → Executing → Reporting → Idle
"""

from typing import Dict, Any, List, Optional
from loguru import logger
from datetime import datetime

from utils.base_agent import BaseAgent
from .models import (
    RoleEnum, RankEnum, MOSEnum, StatusEnum, Task, Order, SitRep, CasRep,
    AgentRecord, Assignment, LIFECYCLE_TRANSITIONS, VALID_COMMAND_TARGETS,
    COMMANDER_ONLY_ROLES,
)
from .general_mind import StrategicMind


class RRAgent(BaseAgent):
    """
    Rational Reserve Agent with Jason hierarchy roles.

    Extends BaseAgent with RoleEnum (primary), optional Rank/MOS metadata,
    chain of command, assignment contracts, and lifecycle transitions.
    """

    def __init__(
        self,
        agent_id: str,
        name: str,
        role: RoleEnum,
        llm_clients: Dict[str, Any],
        commander_id: Optional[str] = None,
        unit_id: Optional[str] = None,
        rank: Optional[RankEnum] = None,
        mos: Optional[MOSEnum] = None,
    ):
        super().__init__(name=name, llm_clients=llm_clients)
        self.agent_id = agent_id
        self.role = role
        # Optional specialty metadata (do not drive CoC)
        self.rank = rank or self._default_rank_for_role(role)
        self.mos = mos or MOSEnum.INFANTRY
        self.commander_id = commander_id
        self.unit_id = unit_id
        self.subordinates: List[str] = []
        self.advisors: List[str] = []  # Specialist advisors attached to this Director
        self.advising: Optional[str] = None  # Director this Specialist advises
        self.status = StatusEnum.IDLE
        self.current_mission: Optional[str] = None
        self.current_task: Optional[Task] = None
        self.current_assignment: Optional[Assignment] = None
        self.swarm_id: Optional[str] = None
        self.missions_completed = 0
        self.performance_score = 0.0

        logger.info(
            f"Initialized RR Agent: {self.name} "
            f"[role={self.role.value}] [rank={self.rank.value}] "
            f"[mos={self.mos.value}] (ID: {self.agent_id})"
        )

    @staticmethod
    def _default_rank_for_role(role: RoleEnum) -> RankEnum:
        mapping = {
            RoleEnum.COMMANDER: RankEnum.GENERAL,
            RoleEnum.ORCHESTRATOR: RankEnum.GENERAL,
            RoleEnum.DIRECTOR: RankEnum.COLONEL,
            RoleEnum.SPECIALIST: RankEnum.SPECIALIST,
            RoleEnum.SPECOPS: RankEnum.SERGEANT_MAJOR,
        }
        return mapping.get(role, RankEnum.SPECIALIST)

    def to_record(self) -> AgentRecord:
        return AgentRecord(
            id=self.agent_id,
            name=self.name,
            role=self.role,
            rank=self.rank,
            mos=self.mos,
            unit_id=self.unit_id,
            commander_id=self.commander_id,
            subordinates=self.subordinates,
            advisors=self.advisors,
            advising=self.advising,
            status=self.status,
            current_mission=self.current_mission,
            current_task=self.current_task.id if self.current_task else None,
            current_assignment=self.current_assignment.id if self.current_assignment else None,
            swarm_id=self.swarm_id,
            missions_completed=self.missions_completed,
            performance_score=self.performance_score,
        )

    def transition_status(self, new_status: StatusEnum) -> bool:
        """Apply a lifecycle transition. Returns False if illegal."""
        allowed = LIFECYCLE_TRANSITIONS.get(self.status, set())
        # Normalize aliases to primary members for comparison
        current = StatusEnum(self.status.value)
        target = StatusEnum(new_status.value)
        allowed_values = {s.value for s in allowed}
        if target.value not in allowed_values and target != current:
            logger.warning(
                f"[{self.role.value} {self.name}] Illegal transition "
                f"{self.status.value} → {new_status.value}"
            )
            return False
        self.status = target
        return True

    def receive_order(self, order: Order, sender_role: Optional[RoleEnum] = None) -> bool:
        """
        Receive order/brief from superior or advisory requester.

        Rules:
        - Line orders only from direct commander_id (Commander→Orchestrator→Director).
        - Specialists accept advisory briefs (is_advisory=True) from Orchestrator
          or from the Director they advise; they never accept command orders over
          a Director and never issue line commands.
        - Peer Specialist reassignment is rejected.
        - Directors reject orders from Specialists (Specialists do not command).
        """
        logger.info(
            f"[{self.role.value} {self.name}] Received order from {order.from_agent}: "
            f"{order.order_type} (advisory={order.is_advisory})"
        )

        # SpecOps sits outside the line chain: it takes orders from the
        # Commander only, and never issues line orders to anyone.
        if self.role in COMMANDER_ONLY_ROLES:
            if sender_role != RoleEnum.COMMANDER or (
                self.commander_id is not None and order.from_agent != self.commander_id
            ):
                logger.warning(
                    f"[{self.role.value} {self.name}] Order rejected: "
                    f"SpecOps takes orders from the Commander only"
                )
                return False
        if sender_role in COMMANDER_ONLY_ROLES:
            logger.warning(
                f"[{self.role.value} {self.name}] Order rejected: "
                f"SpecOps does not command the line"
            )
            return False

        # Reject Specialist trying to command a Director / anyone
        if sender_role == RoleEnum.SPECIALIST and not order.is_advisory:
            logger.warning(
                f"[{self.role.value} {self.name}] Order rejected: "
                f"Specialists do not issue line commands"
            )
            return False

        # Directors never take line orders from Specialists
        if self.role == RoleEnum.DIRECTOR and sender_role == RoleEnum.SPECIALIST:
            logger.warning(
                f"[{self.role.value} {self.name}] Order rejected: "
                f"Specialist cannot command Director"
            )
            return False

        # Peer Specialist cannot reassign another Specialist
        if (
            self.role == RoleEnum.SPECIALIST
            and sender_role == RoleEnum.SPECIALIST
            and order.from_agent != self.commander_id
        ):
            logger.warning(
                f"[{self.role.value} {self.name}] Order rejected: "
                f"peer Specialist reassignment not allowed"
            )
            return False

        # Specialist advisory path
        if self.role == RoleEnum.SPECIALIST:
            if order.is_advisory or order.order_type == "advisory_brief":
                # Accept advisory brief from Orchestrator or advised Director
                allowed_senders = {
                    s for s in (self.commander_id, self.advising) if s is not None
                }
                if order.from_agent in allowed_senders:
                    order.acknowledged_at = datetime.utcnow()
                    self.current_assignment = order.to_assignment()
                    self.transition_status(StatusEnum.BRIEFED)
                    logger.info(f"[{self.role.value} {self.name}] Advisory brief acknowledged")
                    return True
                logger.warning(
                    f"[{self.role.value} {self.name}] Advisory brief rejected: "
                    f"not from Orchestrator/Director attach"
                )
                return False
            # Non-advisory to Specialist: only from direct commander (Orchestrator)
            if self.commander_id is not None and order.from_agent != self.commander_id:
                logger.warning(
                    f"[{self.role.value} {self.name}] Order rejected: not from commander"
                )
                return False

        # Line-authority path: must be from direct commander
        if order.from_agent != self.commander_id and self.commander_id is not None:
            logger.warning(
                f"[{self.role.value} {self.name}] Order rejected: "
                f"not from direct commander"
            )
            return False

        # Optional role-target validation when sender_role provided
        if sender_role is not None and not order.is_advisory:
            allowed_targets = VALID_COMMAND_TARGETS.get(sender_role, set())
            if allowed_targets and self.role not in allowed_targets:
                # Allow Commander→Orchestrator, Orchestrator→Director only
                # Specialists are not valid line targets for command
                if self.role == RoleEnum.SPECIALIST and sender_role == RoleEnum.ORCHESTRATOR:
                    pass  # Orchestrator may engage Specialist as advisor (prefer is_advisory)
                elif self.role not in allowed_targets:
                    logger.warning(
                        f"[{self.role.value} {self.name}] Order rejected: "
                        f"{sender_role.value} cannot line-command {self.role.value}"
                    )
                    return False

        assignment = order.to_assignment()
        if order.order_type == "assignment":
            missing = assignment.missing_fields()
            if missing:
                logger.warning(
                    f"[{self.role.value} {self.name}] Assignment rejected: "
                    f"missing contract fields {missing}"
                )
                return False

        if not self.transition_status(StatusEnum.BRIEFED):
            return False
        order.acknowledged_at = datetime.utcnow()
        self.current_assignment = assignment
        logger.info(f"[{self.role.value} {self.name}] Order acknowledged → BRIEFED")
        return True

    def execute_task(self, task: Task) -> Any:
        """Execute assigned task using LLM; drive lifecycle EXECUTING → REPORTING → IDLE."""
        if StatusEnum(self.status.value) not in (StatusEnum.BRIEFED, StatusEnum.BLOCKED):
            raise RuntimeError(
                f"{self.role.value} {self.name} must be briefed before executing "
                f"(status={self.status.value})"
            )
        if not self.transition_status(StatusEnum.EXECUTING):
            raise RuntimeError(f"Illegal transition {self.status.value} → executing")
        self.current_task = task
        task.status = StatusEnum.EXECUTING

        logger.info(
            f"[{self.role.value} {self.name}] Executing task: {task.description}"
        )

        try:
            llm_name = self._select_llm_for_role()
            prompt = self._build_task_prompt(task)
            result = self.query_llm(llm_name, prompt)

            task.status = StatusEnum.REPORTING
            task.result = result
            task.completed_at = datetime.utcnow()
            self.transition_status(StatusEnum.REPORTING)
            # After reporting, return to IDLE and clear the assignment
            self.transition_status(StatusEnum.IDLE)
            self.current_task = None
            self.current_assignment = None
            self.missions_completed += 1

            logger.info(f"[{self.role.value} {self.name}] Task completed → IDLE")
            return result

        except Exception as e:
            logger.error(f"[{self.role.value} {self.name}] Task failed: {str(e)}")
            task.status = StatusEnum.FAILED
            task.error = str(e)
            self.status = StatusEnum.FAILED
            raise

    def delegate_task(
        self,
        task: Task,
        subordinate_id: str,
        *,
        goal: Optional[str] = None,
        scope_boundary: str = "",
        done_criteria: Optional[List[str]] = None,
        stop_list: Optional[List[str]] = None,
        role_name: Optional[str] = None,
        is_advisory: bool = False,
    ) -> Order:
        """
        Delegate task / assignment to subordinate or Specialist advisor.

        - Orchestrator → Director (line)
        - Director → sub-agents (subordinates list)
        - Orchestrator/Director → Specialist (advisory brief, is_advisory=True)
        - Specialist cannot delegate line commands to Directors
        """
        if self.role in COMMANDER_ONLY_ROLES:
            raise ValueError("SpecOps does not delegate; it reports to the Commander only")
        if self.role == RoleEnum.SPECIALIST and not is_advisory:
            raise ValueError(
                "Specialists do not issue line commands; advisory briefs only"
            )

        if not is_advisory and subordinate_id not in self.subordinates:
            # Allow advisory attach without being in subordinates
            raise ValueError(
                f"Agent {subordinate_id} is not a subordinate of {self.agent_id}"
            )

        if is_advisory and subordinate_id not in self.advisors and subordinate_id not in self.subordinates:
            # Caller should attach advisor first; still allow explicit advisory flag
            pass

        logger.info(
            f"[{self.role.value} {self.name}] Delegating to {subordinate_id}: "
            f"{task.description} (advisory={is_advisory})"
        )

        order = Order(
            from_agent=self.agent_id,
            to_agent=subordinate_id,
            order_type="advisory_brief" if is_advisory else "assignment",
            content=task.description,
            task_id=task.id,
            mission_id=self.current_mission,
            goal=goal or task.description,
            scope_boundary=scope_boundary,
            done_criteria=done_criteria or [],
            stop_list=stop_list or [],
            role_name=role_name,
            is_advisory=is_advisory,
        )
        if not is_advisory:
            task.assigned_to = subordinate_id
        return order

    def attach_advisor(self, specialist_id: str):
        """Director (or Orchestrator) attaches a Specialist as freelance advisor."""
        if specialist_id not in self.advisors:
            self.advisors.append(specialist_id)
            logger.info(
                f"[{self.role.value} {self.name}] Attached advisor: {specialist_id}"
            )

    def report_sitrep(
        self,
        *,
        kind: str = "status",
        done: Optional[List[str]] = None,
        next_actions: Optional[List[str]] = None,
        blockers: Optional[List[str]] = None,
        message: Optional[str] = None,
    ) -> SitRep:
        """Generate done/blocked/next style SitRep for superior."""
        progress = 0.0
        if kind == "done":
            progress = 1.0
        elif self.current_task:
            if self.status == StatusEnum.EXECUTING:
                progress = 0.5
            elif self.status in (StatusEnum.REPORTING, StatusEnum.IDLE):
                progress = 1.0

        block_list = list(blockers or [])
        if self.status == StatusEnum.BLOCKED and not block_list:
            block_list.append("Agent is blocked on current task")

        if kind == "done":
            self.transition_status(StatusEnum.REPORTING)
        elif kind == "blocked":
            self.status = StatusEnum.BLOCKED

        sitrep = SitRep(
            from_agent=self.agent_id,
            to_agent=self.commander_id,
            status=self.status,
            current_task=self.current_task.id if self.current_task else None,
            progress=progress,
            message=message or self._generate_sitrep_message(),
            blockers=block_list,
            done=list(done or []),
            next_actions=list(next_actions or []),
            report_kind=kind,
        )
        logger.info(f"[{self.role.value} {self.name}] SITREP ({kind}) generated")
        if kind == "done":
            self.transition_status(StatusEnum.IDLE)
        return sitrep

    def report_casrep(self, error: Exception, task: Optional[Task] = None) -> CasRep:
        casrep = CasRep(
            agent_id=self.agent_id,
            failure_type="error",
            error_message=str(error),
            task_id=task.id if task else None,
        )
        self.status = StatusEnum.FAILED
        logger.error(f"[{self.role.value} {self.name}] CASREP: {str(error)}")
        return casrep

    def process(self, input_data: Any) -> Any:
        if isinstance(input_data, Task):
            return self.execute_task(input_data)
        elif isinstance(input_data, Order):
            accepted = self.receive_order(input_data)
            return {"acknowledged": accepted}
        else:
            raise ValueError(f"RRAgent cannot process input type: {type(input_data)}")

    def _select_llm_for_role(self) -> str:
        role_llm_map = {
            RoleEnum.COMMANDER: "anthropic",
            RoleEnum.ORCHESTRATOR: "anthropic",
            RoleEnum.DIRECTOR: "anthropic",
            RoleEnum.SPECIALIST: "llama",
        }
        llm_name = role_llm_map.get(self.role, "groq")
        if llm_name not in self.llm_clients:
            available = list(self.llm_clients.keys())
            llm_name = available[0] if available else "openai"
            logger.warning(
                f"[{self.role.value} {self.name}] Preferred LLM not available, "
                f"using {llm_name}"
            )
        return llm_name

    def _build_task_prompt(self, task: Task) -> str:
        if self.role in (RoleEnum.COMMANDER, RoleEnum.ORCHESTRATOR):
            context = {
                "task": task.description,
                "task_metadata": task.metadata,
                "agent_name": self.name,
                "role": self.role.value,
                "swarm_id": self.swarm_id,
                "mission_id": self.current_mission,
                "status": self.status.value,
            }
            return StrategicMind.unified_strategic_prompt(
                objective=task.description,
                context=context,
                mode="full",
            )

        role_label = self.role.value.upper()
        prompt = f"""You are a {role_label} in the Rational Reserve.
Optional specialty: rank={self.rank.value}, MOS={self.mos.value} ({self.mos.name}).

Task: {task.description}
"""
        if self.current_assignment:
            a = self.current_assignment
            prompt += f"""
Assignment contract:
- Goal: {a.goal}
- Scope boundary: {a.scope_boundary}
- Done criteria: {a.done_criteria}
- Stop list: {a.stop_list}
"""
        if self.role == RoleEnum.SPECIALIST:
            prompt += "\nYou are a freelance advisor. Provide expert counsel; do not issue orders to Directors.\n"
        elif self.role == RoleEnum.DIRECTOR:
            prompt += "\nYou own this workstream. Operationalize the goal; you may request Specialist advisors.\n"

        mos_guidance = {
            MOSEnum.INTELLIGENCE_ANALYST: "Analyze data, identify patterns, provide intelligence assessment.",
            MOSEnum.COUNTERINTELLIGENCE: "Detect threats, validate security, identify anomalies.",
            MOSEnum.SIGINT_ANALYST: "Process signals, analyze logs, extract intelligence.",
            MOSEnum.INFANTRY: "Execute core task directly and efficiently.",
            MOSEnum.COMBAT_ENGINEER: "Build, construct, generate code or systems.",
            MOSEnum.ARTILLERY: "Process large batches, heavy computation.",
            MOSEnum.CAVALRY_SCOUT: "Explore, reconnoiter, gather information.",
            MOSEnum.IT_SPECIALIST: "Configure systems, manage infrastructure.",
            MOSEnum.HR_SPECIALIST: "Manage resources, track personnel.",
            MOSEnum.TRANSPORT: "Move data, orchestrate transfers.",
            MOSEnum.LOGISTICS: "Allocate resources, manage dependencies.",
            MOSEnum.SF_ENGINEER: "Design advanced solutions, solve complex problems.",
            MOSEnum.RANGER: "Rapid execution, emergency response.",
            MOSEnum.SOAR: "High-speed processing, real-time operations.",
        }
        if self.mos in mos_guidance:
            prompt += f"\nSpecialty guidance: {mos_guidance[self.mos]}\n"
        return prompt

    def _generate_sitrep_message(self) -> str:
        if self.current_task:
            return f"Currently executing: {self.current_task.description}"
        if self.status == StatusEnum.IDLE:
            return "Idle — standing by for assignment"
        if self.status == StatusEnum.BRIEFED:
            return "Briefed — ready to execute"
        if self.status == StatusEnum.EXECUTING:
            return "Executing assignment"
        if self.status == StatusEnum.REPORTING:
            return "Reporting results"
        return f"Status: {self.status.value}"

    def add_subordinate(self, subordinate_id: str):
        if subordinate_id not in self.subordinates:
            self.subordinates.append(subordinate_id)
            logger.info(
                f"[{self.role.value} {self.name}] Added subordinate: {subordinate_id}"
            )

    def remove_subordinate(self, subordinate_id: str):
        if subordinate_id in self.subordinates:
            self.subordinates.remove(subordinate_id)

    def is_line_authority(self) -> bool:
        """True if role is on the command line (not Specialist advisor)."""
        return self.role in (
            RoleEnum.COMMANDER, RoleEnum.ORCHESTRATOR, RoleEnum.DIRECTOR
        )

    def can_command_role(self, target_role: RoleEnum) -> bool:
        """Check line-authority: can this role issue orders to target_role?"""
        return target_role in VALID_COMMAND_TARGETS.get(self.role, set())

    # Legacy helpers kept for RankEnum metadata compatibility
    def is_officer(self) -> bool:
        return self.rank in [
            RankEnum.GENERAL, RankEnum.COLONEL, RankEnum.MAJOR,
            RankEnum.CAPTAIN, RankEnum.LIEUTENANT,
        ]

    def is_nco(self) -> bool:
        return self.rank in [
            RankEnum.SERGEANT_MAJOR, RankEnum.MASTER_SERGEANT,
            RankEnum.SERGEANT, RankEnum.CORPORAL,
        ]

    def is_enlisted(self) -> bool:
        return self.rank in [
            RankEnum.SPECIALIST, RankEnum.PRIVATE_FIRST_CLASS, RankEnum.PRIVATE,
        ]

    def can_command(self, subordinate_rank: RankEnum) -> bool:
        """Legacy rank-based check (metadata only; prefer can_command_role)."""
        rank_hierarchy = [
            RankEnum.PRIVATE, RankEnum.PRIVATE_FIRST_CLASS, RankEnum.SPECIALIST,
            RankEnum.CORPORAL, RankEnum.SERGEANT, RankEnum.MASTER_SERGEANT,
            RankEnum.SERGEANT_MAJOR, RankEnum.LIEUTENANT, RankEnum.CAPTAIN,
            RankEnum.MAJOR, RankEnum.COLONEL, RankEnum.GENERAL,
        ]
        return rank_hierarchy.index(self.rank) > rank_hierarchy.index(subordinate_rank)
