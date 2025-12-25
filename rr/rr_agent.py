"""
RRAgent - Rational Reserve Agent with military rank and MOS
"""

from typing import Dict, Any, List, Optional
from loguru import logger
from datetime import datetime

from utils.base_agent import BaseAgent
from .models import (
    RankEnum, MOSEnum, StatusEnum, Task, Order, SitRep, CasRep, AgentRecord
)
from .general_mind import GeneralMind


class RRAgent(BaseAgent):
    """
    Rational Reserve Agent with military structure

    Extends BaseAgent with rank, MOS, and chain of command capabilities
    """

    def __init__(
        self,
        agent_id: str,
        name: str,
        rank: RankEnum,
        mos: MOSEnum,
        llm_clients: Dict[str, Any],
        commander_id: Optional[str] = None,
        unit_id: Optional[str] = None,
    ):
        super().__init__(name=name, llm_clients=llm_clients)
        self.agent_id = agent_id
        self.rank = rank
        self.mos = mos
        self.commander_id = commander_id
        self.unit_id = unit_id
        self.subordinates: List[str] = []
        self.status = StatusEnum.STANDBY
        self.current_mission: Optional[str] = None
        self.current_task: Optional[Task] = None
        self.swarm_id: Optional[str] = None
        self.missions_completed = 0
        self.performance_score = 0.0

        logger.info(
            f"Initialized RR Agent: {self.name} "
            f"[{self.rank.value}] [{self.mos.value}] (ID: {self.agent_id})"
        )

    def to_record(self) -> AgentRecord:
        """Convert agent to database record"""
        return AgentRecord(
            id=self.agent_id,
            name=self.name,
            rank=self.rank,
            mos=self.mos,
            unit_id=self.unit_id,
            commander_id=self.commander_id,
            subordinates=self.subordinates,
            status=self.status,
            current_mission=self.current_mission,
            current_task=self.current_task.id if self.current_task else None,
            swarm_id=self.swarm_id,
            missions_completed=self.missions_completed,
            performance_score=self.performance_score,
        )

    def receive_order(self, order: Order) -> bool:
        """
        Receive order from superior officer

        Returns True if order accepted, False if rejected
        """
        logger.info(
            f"[{self.rank.value} {self.name}] Received order from {order.from_agent}: "
            f"{order.order_type}"
        )

        # Validate chain of command
        if order.from_agent != self.commander_id and self.commander_id is not None:
            logger.warning(
                f"[{self.rank.value} {self.name}] Order rejected: "
                f"not from direct commander"
            )
            return False

        # Acknowledge order
        order.acknowledged_at = datetime.utcnow()
        logger.info(f"[{self.rank.value} {self.name}] Order acknowledged")
        return True

    def execute_task(self, task: Task) -> Any:
        """
        Execute assigned task using LLM

        Implementation varies by MOS specialty
        """
        self.current_task = task
        self.status = StatusEnum.ENGAGED
        task.status = StatusEnum.ENGAGED

        logger.info(
            f"[{self.rank.value} {self.name}] Executing task: {task.description}"
        )

        try:
            # Select LLM based on rank
            llm_name = self._select_llm_for_rank()

            # Build prompt based on MOS specialty
            prompt = self._build_task_prompt(task)

            # Execute via LLM
            result = self.query_llm(llm_name, prompt)

            # Mark task complete
            task.status = StatusEnum.COMPLETE
            task.result = result
            task.completed_at = datetime.utcnow()
            self.status = StatusEnum.ACTIVE
            self.current_task = None

            logger.info(
                f"[{self.rank.value} {self.name}] Task completed successfully"
            )
            return result

        except Exception as e:
            logger.error(
                f"[{self.rank.value} {self.name}] Task failed: {str(e)}"
            )
            task.status = StatusEnum.FAILED
            task.error = str(e)
            self.status = StatusEnum.ACTIVE
            raise

    def delegate_task(self, task: Task, subordinate_id: str) -> Order:
        """
        Delegate task to subordinate

        Officers and NCOs can delegate to their subordinates
        """
        if subordinate_id not in self.subordinates:
            raise ValueError(
                f"Agent {subordinate_id} is not a subordinate of {self.agent_id}"
            )

        logger.info(
            f"[{self.rank.value} {self.name}] Delegating task to {subordinate_id}: "
            f"{task.description}"
        )

        order = Order(
            from_agent=self.agent_id,
            to_agent=subordinate_id,
            order_type="task_assignment",
            content=task.description,
            task_id=task.id,
            mission_id=self.current_mission,
        )

        task.assigned_to = subordinate_id
        return order

    def report_sitrep(self) -> SitRep:
        """
        Generate situation report for superior
        """
        progress = 0.0
        if self.current_task:
            # Simple progress estimation - can be enhanced
            progress = 0.5 if self.current_task.status == StatusEnum.ENGAGED else 1.0

        blockers = []
        if self.status == StatusEnum.BLOCKED:
            blockers.append("Agent is blocked on current task")

        sitrep = SitRep(
            from_agent=self.agent_id,
            to_agent=self.commander_id,
            status=self.status,
            current_task=self.current_task.id if self.current_task else None,
            progress=progress,
            message=self._generate_sitrep_message(),
            blockers=blockers,
        )

        logger.info(f"[{self.rank.value} {self.name}] SITREP generated")
        return sitrep

    def report_casrep(self, error: Exception, task: Optional[Task] = None) -> CasRep:
        """
        Generate casualty report for failure
        """
        casrep = CasRep(
            agent_id=self.agent_id,
            failure_type="error",
            error_message=str(error),
            task_id=task.id if task else None,
            stack_trace=None,  # Can add traceback here
        )

        self.status = StatusEnum.FAILED
        logger.error(f"[{self.rank.value} {self.name}] CASREP generated: {str(error)}")
        return casrep

    def process(self, input_data: Any) -> Any:
        """
        BaseAgent abstract method implementation

        For RRAgent, processing is task-driven via execute_task
        """
        if isinstance(input_data, Task):
            return self.execute_task(input_data)
        elif isinstance(input_data, Order):
            self.receive_order(input_data)
            return {"acknowledged": True}
        else:
            raise ValueError(f"RRAgent cannot process input type: {type(input_data)}")

    def _select_llm_for_rank(self) -> str:
        """
        Select appropriate LLM based on rank

        Higher ranks get more powerful models
        """
        rank_llm_map = {
            # Officers get premium models
            RankEnum.GENERAL: "anthropic",  # Claude Sonnet 4
            RankEnum.COLONEL: "anthropic",
            RankEnum.MAJOR: "anthropic",
            RankEnum.CAPTAIN: "llama",  # Llama3 8B local
            RankEnum.LIEUTENANT: "llama",

            # NCOs get local models
            RankEnum.SERGEANT_MAJOR: "llama",
            RankEnum.MASTER_SERGEANT: "llama",
            RankEnum.SERGEANT: "llama",
            RankEnum.CORPORAL: "llama",

            # Enlisted get local models
            RankEnum.SPECIALIST: "llama",
            RankEnum.PRIVATE_FIRST_CLASS: "llama",
            RankEnum.PRIVATE: "llama",
        }

        llm_name = rank_llm_map.get(self.rank, "groq")

        # Fallback if LLM not available
        if llm_name not in self.llm_clients:
            available = list(self.llm_clients.keys())
            llm_name = available[0] if available else "openai"
            logger.warning(
                f"[{self.rank.value} {self.name}] Preferred LLM not available, "
                f"using {llm_name}"
            )

        return llm_name

    def _build_task_prompt(self, task: Task) -> str:
        """
        Build prompt for task execution based on MOS and rank

        Generals use the enhanced cognitive framework with:
        - Recursive/iterative thinking
        - Reactive complex (pivot capability)
        - Predictive engine (event horizon analysis)
        """
        # GENERALS USE ENHANCED COGNITIVE FRAMEWORK
        if self.rank == RankEnum.GENERAL:
            context = {
                "task": task.description,
                "task_metadata": task.metadata,
                "agent_name": self.name,
                "swarm_id": self.swarm_id,
                "mission_id": self.current_mission,
                "status": self.status.value,
            }
            return GeneralMind.unified_strategic_prompt(
                objective=task.description,
                context=context,
                mode="full"
            )

        # Standard agents use MOS-based prompts
        prompt = f"""You are a {self.rank.value} ({self.rank.name}) in the Rational Reserve with MOS {self.mos.value} ({self.mos.name}).

Task: {task.description}

Execute this task according to your specialty and rank. Provide a clear, actionable result.
"""

        # Add MOS-specific guidance
        mos_guidance = {
            # Intelligence
            MOSEnum.INTELLIGENCE_ANALYST: "Analyze data, identify patterns, provide intelligence assessment.",
            MOSEnum.COUNTERINTELLIGENCE: "Detect threats, validate security, identify anomalies.",
            MOSEnum.SIGINT_ANALYST: "Process signals, analyze logs, extract intelligence.",

            # Operations
            MOSEnum.INFANTRY: "Execute core task directly and efficiently.",
            MOSEnum.COMBAT_ENGINEER: "Build, construct, generate code or systems.",
            MOSEnum.ARTILLERY: "Process large batches, heavy computation.",
            MOSEnum.CAVALRY_SCOUT: "Explore, reconnoiter, gather information.",

            # Support
            MOSEnum.IT_SPECIALIST: "Configure systems, manage infrastructure.",
            MOSEnum.HR_SPECIALIST: "Manage resources, track personnel.",
            MOSEnum.TRANSPORT: "Move data, orchestrate transfers.",
            MOSEnum.LOGISTICS: "Allocate resources, manage dependencies.",

            # Special Operations
            MOSEnum.SF_ENGINEER: "Design advanced solutions, solve complex problems.",
            MOSEnum.RANGER: "Rapid execution, emergency response.",
            MOSEnum.SOAR: "High-speed processing, real-time operations.",
        }

        if self.mos in mos_guidance:
            prompt += f"\nYour specialty guidance: {mos_guidance[self.mos]}\n"

        return prompt

    def _generate_sitrep_message(self) -> str:
        """Generate human-readable SITREP message"""
        if self.current_task:
            return f"Currently executing: {self.current_task.description}"
        elif self.status == StatusEnum.STANDBY:
            return "Standing by for orders"
        elif self.status == StatusEnum.ACTIVE:
            return "Active and ready for tasking"
        elif self.status == StatusEnum.COMPLETE:
            return "Mission complete"
        else:
            return f"Status: {self.status.value}"

    def add_subordinate(self, subordinate_id: str):
        """Add subordinate to command"""
        if subordinate_id not in self.subordinates:
            self.subordinates.append(subordinate_id)
            logger.info(
                f"[{self.rank.value} {self.name}] Added subordinate: {subordinate_id}"
            )

    def remove_subordinate(self, subordinate_id: str):
        """Remove subordinate from command"""
        if subordinate_id in self.subordinates:
            self.subordinates.remove(subordinate_id)
            logger.info(
                f"[{self.rank.value} {self.name}] Removed subordinate: {subordinate_id}"
            )

    def is_officer(self) -> bool:
        """Check if agent is officer rank"""
        return self.rank in [
            RankEnum.GENERAL, RankEnum.COLONEL, RankEnum.MAJOR,
            RankEnum.CAPTAIN, RankEnum.LIEUTENANT
        ]

    def is_nco(self) -> bool:
        """Check if agent is NCO rank"""
        return self.rank in [
            RankEnum.SERGEANT_MAJOR, RankEnum.MASTER_SERGEANT,
            RankEnum.SERGEANT, RankEnum.CORPORAL
        ]

    def is_enlisted(self) -> bool:
        """Check if agent is enlisted rank"""
        return self.rank in [
            RankEnum.SPECIALIST, RankEnum.PRIVATE_FIRST_CLASS, RankEnum.PRIVATE
        ]

    def can_command(self, subordinate_rank: RankEnum) -> bool:
        """Check if this agent can command an agent of given rank"""
        rank_hierarchy = [
            RankEnum.PRIVATE,
            RankEnum.PRIVATE_FIRST_CLASS,
            RankEnum.SPECIALIST,
            RankEnum.CORPORAL,
            RankEnum.SERGEANT,
            RankEnum.MASTER_SERGEANT,
            RankEnum.SERGEANT_MAJOR,
            RankEnum.LIEUTENANT,
            RankEnum.CAPTAIN,
            RankEnum.MAJOR,
            RankEnum.COLONEL,
            RankEnum.GENERAL,
        ]

        my_level = rank_hierarchy.index(self.rank)
        their_level = rank_hierarchy.index(subordinate_rank)
        return my_level > their_level
