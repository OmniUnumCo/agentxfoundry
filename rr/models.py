"""
Data models for Rational Reserve swarm system

Line authority (RoleEnum chain of command):
  Commander → Orchestrator → Director
  Commander → SpecOps (direct; bypasses Orchestrator and Directors)

Specialists are freelance/on-call advisors to Directors.
They consult; they do not own programs or issue orders to Directors.

RankEnum / MOSEnum remain as optional specialty metadata.
Lifecycle: Idle → Briefed → Executing → Reporting → Idle
"""

from typing import Dict, List, Optional, Any, Set
from pydantic import BaseModel, Field
from enum import Enum
from datetime import datetime
import uuid


class RoleEnum(str, Enum):
    """Primary roles.

    Line authority (highest first): COMMANDER → ORCHESTRATOR → DIRECTOR
    SPECIALIST is advisory (not in the command line above Directors).
    """
    COMMANDER = "commander"
    ORCHESTRATOR = "orchestrator"
    DIRECTOR = "director"
    SPECIALIST = "specialist"
    SPECOPS = "specops"  # reports directly to Commander only


# Line-authority levels (Specialist is advisory — not ranked above Director)
ROLE_LINE_LEVEL = {
    RoleEnum.DIRECTOR: 1,
    RoleEnum.ORCHESTRATOR: 2,
    RoleEnum.COMMANDER: 3,
}


# Direct line-command targets (who may issue orders to whom)
VALID_COMMAND_TARGETS: Dict[RoleEnum, Set[RoleEnum]] = {
    RoleEnum.COMMANDER: {RoleEnum.ORCHESTRATOR, RoleEnum.SPECOPS},
    RoleEnum.ORCHESTRATOR: {RoleEnum.DIRECTOR},
    RoleEnum.DIRECTOR: set(),  # Directors assign sub-agents; may request Specialist advisors
    RoleEnum.SPECIALIST: set(),  # Specialists advise; they do not command
    RoleEnum.SPECOPS: set(),  # SpecOps executes for Commander; commands no one
}

# Roles that may only ever be tasked by (and report to) the Commander.
COMMANDER_ONLY_ROLES: Set[RoleEnum] = {RoleEnum.SPECOPS}

# Assignment contract fields every line assignment must carry.
REQUIRED_ASSIGNMENT_FIELDS = ("goal", "scope_boundary", "done_criteria", "stop_list", "role_name")


class RankEnum(str, Enum):
    """Military ranks — optional specialty metadata (kept for compatibility).

    Chain-of-command and factory spawn paths use RoleEnum, not RankEnum.
    Mapping (legacy → role):
      GEN ≈ COMMANDER or ORCHESTRATOR
      COL/MAJ/CPT ≈ DIRECTOR (owns workstream / operationalizes)
      LT/SGM/MSG/SGT/CPL ≈ DIRECTOR sub-tier or operational support
      SPC specialty experts ≈ SPECIALIST (freelance advisor)
      PFC/PVT ≈ operational sub-agents under Director
    """
    GENERAL = "GEN"
    COLONEL = "COL"
    MAJOR = "MAJ"
    CAPTAIN = "CPT"
    LIEUTENANT = "LT"
    SERGEANT_MAJOR = "SGM"
    MASTER_SERGEANT = "MSG"
    SERGEANT = "SGT"
    CORPORAL = "CPL"
    SPECIALIST = "SPC"
    PRIVATE_FIRST_CLASS = "PFC"
    PRIVATE = "PVT"


class MOSEnum(str, Enum):
    """Military Occupational Specialties — optional specialty metadata"""
    INTELLIGENCE_ANALYST = "35F"
    COUNTERINTELLIGENCE = "35L"
    SIGINT_ANALYST = "35N"
    INFANTRY = "11B"
    COMBAT_ENGINEER = "12B"
    ARTILLERY = "13B"
    CAVALRY_SCOUT = "19D"
    IT_SPECIALIST = "25B"
    HR_SPECIALIST = "42A"
    TRANSPORT = "88M"
    LOGISTICS = "92G"
    SF_ENGINEER = "18B"
    RANGER = "75R"
    SOAR = "160"


class StatusEnum(str, Enum):
    """Agent and mission lifecycle status states.

    Primary lifecycle: IDLE → BRIEFED → EXECUTING → REPORTING → IDLE

    Legacy aliases (same values for compatibility):
      STANDBY → IDLE
      ACTIVE / ENGAGED → EXECUTING
      COMPLETE → REPORTING (then return to IDLE after SitRep)
      FAILED / BLOCKED remain
    """
    IDLE = "idle"
    BRIEFED = "briefed"
    EXECUTING = "executing"
    REPORTING = "reporting"
    FAILED = "failed"
    BLOCKED = "blocked"
    # Legacy aliases
    STANDBY = "idle"
    ACTIVE = "executing"
    ENGAGED = "executing"
    COMPLETE = "reporting"


LIFECYCLE_TRANSITIONS = {
    StatusEnum.IDLE: {StatusEnum.BRIEFED, StatusEnum.FAILED},
    StatusEnum.BRIEFED: {StatusEnum.EXECUTING, StatusEnum.IDLE, StatusEnum.BLOCKED, StatusEnum.FAILED},
    StatusEnum.EXECUTING: {StatusEnum.REPORTING, StatusEnum.BLOCKED, StatusEnum.FAILED},
    StatusEnum.REPORTING: {StatusEnum.IDLE, StatusEnum.FAILED},
    StatusEnum.BLOCKED: {StatusEnum.EXECUTING, StatusEnum.REPORTING, StatusEnum.IDLE, StatusEnum.FAILED},
    StatusEnum.FAILED: {StatusEnum.IDLE},
}


class PriorityEnum(str, Enum):
    ROUTINE = "routine"
    PRIORITY = "priority"
    URGENT = "urgent"
    CRITICAL = "critical"


class Assignment(BaseModel):
    """Assignment contract handed down the chain (or advisory brief to Specialist).

    Fields: goal, scope_boundary, done_criteria, stop_list, role_name
    """
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    goal: str
    scope_boundary: str = ""
    done_criteria: List[str] = Field(default_factory=list)
    stop_list: List[str] = Field(default_factory=list)
    role_name: Optional[str] = None
    from_agent: Optional[str] = None
    to_agent: Optional[str] = None
    mission_id: Optional[str] = None
    is_advisory: bool = False  # True when briefing a Specialist advisor
    created_at: datetime = Field(default_factory=datetime.utcnow)
    metadata: Dict[str, Any] = Field(default_factory=dict)

    def missing_fields(self) -> List[str]:
        """Contract fields that are empty (goal, scope, done, stop list, role)."""
        return [f for f in REQUIRED_ASSIGNMENT_FIELDS if not getattr(self, f)]

    def is_complete(self) -> bool:
        return not self.missing_fields()


class Task(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    description: str
    assigned_to: Optional[str] = None
    status: StatusEnum = StatusEnum.IDLE
    dependencies: List[str] = Field(default_factory=list)
    result: Optional[Any] = None
    error: Optional[str] = None
    created_at: datetime = Field(default_factory=datetime.utcnow)
    completed_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Mission(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    objective: str
    description: Optional[str] = None
    priority: PriorityEnum = PriorityEnum.ROUTINE
    status: StatusEnum = StatusEnum.IDLE
    constraints: Dict[str, Any] = Field(default_factory=dict)
    resources: Dict[str, Any] = Field(default_factory=dict)
    tasks: List[Task] = Field(default_factory=list)
    assigned_swarm: Optional[str] = None
    created_at: datetime = Field(default_factory=datetime.utcnow)
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    result: Optional[Any] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Order(BaseModel):
    """Command or advisory brief from one agent to another.

    Includes Assignment contract fields: goal, scope_boundary, done_criteria,
    stop_list, role_name.
    """
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    from_agent: str
    to_agent: str
    order_type: str  # "mission", "frago", "status_request", "assignment", "advisory_brief"
    content: str
    task_id: Optional[str] = None
    mission_id: Optional[str] = None
    goal: Optional[str] = None
    scope_boundary: Optional[str] = None
    done_criteria: List[str] = Field(default_factory=list)
    stop_list: List[str] = Field(default_factory=list)
    role_name: Optional[str] = None
    is_advisory: bool = False
    issued_at: datetime = Field(default_factory=datetime.utcnow)
    acknowledged_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)

    def to_assignment(self) -> "Assignment":
        return Assignment(
            goal=self.goal or self.content,
            scope_boundary=self.scope_boundary or "",
            done_criteria=list(self.done_criteria),
            stop_list=list(self.stop_list),
            role_name=self.role_name,
            from_agent=self.from_agent,
            to_agent=self.to_agent,
            mission_id=self.mission_id,
            is_advisory=self.is_advisory,
            metadata=dict(self.metadata),
        )


class SitRep(BaseModel):
    """Situation Report — done / blocked / next style."""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    from_agent: str
    to_agent: Optional[str] = None
    status: StatusEnum
    current_task: Optional[str] = None
    progress: float = 0.0
    message: str
    blockers: List[str] = Field(default_factory=list)
    done: List[str] = Field(default_factory=list)
    next_actions: List[str] = Field(default_factory=list)
    report_kind: str = "status"  # "done" | "blocked" | "next" | "status"
    reported_at: datetime = Field(default_factory=datetime.utcnow)
    metadata: Dict[str, Any] = Field(default_factory=dict)


class CasRep(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    failure_type: str
    error_message: str
    task_id: Optional[str] = None
    stack_trace: Optional[str] = None
    reported_at: datetime = Field(default_factory=datetime.utcnow)
    recovery_action: Optional[str] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class AAR(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    mission_id: str
    swarm_id: str
    success: bool
    objectives_met: List[str] = Field(default_factory=list)
    objectives_missed: List[str] = Field(default_factory=list)
    lessons_learned: List[str] = Field(default_factory=list)
    agent_performance: Dict[str, Any] = Field(default_factory=dict)
    resource_usage: Dict[str, Any] = Field(default_factory=dict)
    timeline: Dict[str, datetime] = Field(default_factory=dict)
    recommendations: List[str] = Field(default_factory=list)
    created_at: datetime = Field(default_factory=datetime.utcnow)
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Unit(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    name: str
    unit_type: str
    commander: Optional[str] = None
    members: List[str] = Field(default_factory=list)
    parent_unit: Optional[str] = None
    sub_units: List[str] = Field(default_factory=list)
    status: StatusEnum = StatusEnum.IDLE
    created_at: datetime = Field(default_factory=datetime.utcnow)
    disbanded_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Swarm(BaseModel):
    """Swarm: optional Commander → Orchestrator → Directors (+ Specialist advisors)."""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    name: str
    mission_id: str
    general_id: Optional[str] = None  # legacy alias → orchestrator_id
    commander_id: Optional[str] = None
    orchestrator_id: Optional[str] = None
    agents: List[str] = Field(default_factory=list)
    units: List[str] = Field(default_factory=list)
    status: StatusEnum = StatusEnum.IDLE
    created_at: datetime = Field(default_factory=datetime.utcnow)
    activated_at: Optional[datetime] = None
    disbanded_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class AgentRecord(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    name: str
    role: RoleEnum = RoleEnum.DIRECTOR
    rank: Optional[RankEnum] = None
    mos: Optional[MOSEnum] = None
    unit_id: Optional[str] = None
    commander_id: Optional[str] = None  # direct line superior (None for advisory-only attach)
    subordinates: List[str] = Field(default_factory=list)
    advisors: List[str] = Field(default_factory=list)  # Specialist advisor agent_ids
    advising: Optional[str] = None  # for Specialists: Director they advise
    status: StatusEnum = StatusEnum.IDLE
    current_mission: Optional[str] = None
    current_task: Optional[str] = None
    current_assignment: Optional[str] = None
    swarm_id: Optional[str] = None
    created_at: datetime = Field(default_factory=datetime.utcnow)
    activated_at: Optional[datetime] = None
    deactivated_at: Optional[datetime] = None
    missions_completed: int = 0
    performance_score: float = 0.0
    metadata: Dict[str, Any] = Field(default_factory=dict)
