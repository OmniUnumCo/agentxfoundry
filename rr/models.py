"""
Data models for Rational Reserve swarm system
"""

from typing import Dict, List, Optional, Any
from pydantic import BaseModel, Field
from enum import Enum
from datetime import datetime
import uuid


class RankEnum(str, Enum):
    """Military ranks in Rational Reserve"""
    # Officers
    GENERAL = "GEN"
    COLONEL = "COL"
    MAJOR = "MAJ"
    CAPTAIN = "CPT"
    LIEUTENANT = "LT"

    # NCOs
    SERGEANT_MAJOR = "SGM"
    MASTER_SERGEANT = "MSG"
    SERGEANT = "SGT"
    CORPORAL = "CPL"

    # Enlisted
    SPECIALIST = "SPC"
    PRIVATE_FIRST_CLASS = "PFC"
    PRIVATE = "PVT"


class MOSEnum(str, Enum):
    """Military Occupational Specialties"""
    # Intelligence
    INTELLIGENCE_ANALYST = "35F"
    COUNTERINTELLIGENCE = "35L"
    SIGINT_ANALYST = "35N"

    # Operations
    INFANTRY = "11B"
    COMBAT_ENGINEER = "12B"
    ARTILLERY = "13B"
    CAVALRY_SCOUT = "19D"

    # Support
    IT_SPECIALIST = "25B"
    HR_SPECIALIST = "42A"
    TRANSPORT = "88M"
    LOGISTICS = "92G"

    # Special Operations
    SF_ENGINEER = "18B"
    RANGER = "75R"
    SOAR = "160"


class StatusEnum(str, Enum):
    """Agent and mission status states"""
    STANDBY = "standby"
    ACTIVE = "active"
    ENGAGED = "engaged"
    COMPLETE = "complete"
    FAILED = "failed"
    BLOCKED = "blocked"


class PriorityEnum(str, Enum):
    """Mission priority levels"""
    ROUTINE = "routine"
    PRIORITY = "priority"
    URGENT = "urgent"
    CRITICAL = "critical"


class Task(BaseModel):
    """Atomic work unit"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    description: str
    assigned_to: Optional[str] = None  # agent_id
    status: StatusEnum = StatusEnum.STANDBY
    dependencies: List[str] = Field(default_factory=list)  # task_ids
    result: Optional[Any] = None
    error: Optional[str] = None
    created_at: datetime = Field(default_factory=datetime.utcnow)
    completed_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Mission(BaseModel):
    """High-level objective with decomposed tasks"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    objective: str
    description: Optional[str] = None
    priority: PriorityEnum = PriorityEnum.ROUTINE
    status: StatusEnum = StatusEnum.STANDBY
    constraints: Dict[str, Any] = Field(default_factory=dict)
    resources: Dict[str, Any] = Field(default_factory=dict)
    tasks: List[Task] = Field(default_factory=list)
    assigned_swarm: Optional[str] = None  # swarm_id
    created_at: datetime = Field(default_factory=datetime.utcnow)
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    result: Optional[Any] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Order(BaseModel):
    """Command from superior to subordinate"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    from_agent: str  # superior agent_id
    to_agent: str  # subordinate agent_id
    order_type: str  # "mission", "frago", "status_request"
    content: str
    task_id: Optional[str] = None
    mission_id: Optional[str] = None
    issued_at: datetime = Field(default_factory=datetime.utcnow)
    acknowledged_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class SitRep(BaseModel):
    """Situation Report from subordinate to superior"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    from_agent: str  # reporting agent_id
    to_agent: Optional[str] = None  # receiving agent_id (None = broadcast)
    status: StatusEnum
    current_task: Optional[str] = None  # task_id
    progress: float = 0.0  # 0.0 to 1.0
    message: str
    blockers: List[str] = Field(default_factory=list)
    reported_at: datetime = Field(default_factory=datetime.utcnow)
    metadata: Dict[str, Any] = Field(default_factory=dict)


class CasRep(BaseModel):
    """Casualty Report - agent failure notification"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    failure_type: str  # "error", "timeout", "resource_exhaustion"
    error_message: str
    task_id: Optional[str] = None
    stack_trace: Optional[str] = None
    reported_at: datetime = Field(default_factory=datetime.utcnow)
    recovery_action: Optional[str] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class AAR(BaseModel):
    """After Action Review - post-mission analysis"""
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
    """Organizational unit (squad, platoon, company)"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    name: str  # "Alpha Company", "1st Platoon", etc.
    unit_type: str  # "fire_team", "squad", "platoon", "company", "battalion"
    commander: Optional[str] = None  # agent_id
    members: List[str] = Field(default_factory=list)  # agent_ids
    parent_unit: Optional[str] = None  # parent unit_id
    sub_units: List[str] = Field(default_factory=list)  # child unit_ids
    status: StatusEnum = StatusEnum.STANDBY
    created_at: datetime = Field(default_factory=datetime.utcnow)
    disbanded_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Swarm(BaseModel):
    """Complete swarm instance"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    name: str
    mission_id: str
    general_id: Optional[str] = None  # commanding General agent_id
    agents: List[str] = Field(default_factory=list)  # all agent_ids
    units: List[str] = Field(default_factory=list)  # unit_ids
    status: StatusEnum = StatusEnum.STANDBY
    created_at: datetime = Field(default_factory=datetime.utcnow)
    activated_at: Optional[datetime] = None
    disbanded_at: Optional[datetime] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class AgentRecord(BaseModel):
    """Persistent agent record for database"""
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    name: str
    rank: RankEnum
    mos: MOSEnum
    unit_id: Optional[str] = None
    commander_id: Optional[str] = None  # superior agent_id
    subordinates: List[str] = Field(default_factory=list)  # agent_ids
    status: StatusEnum = StatusEnum.STANDBY
    current_mission: Optional[str] = None  # mission_id
    current_task: Optional[str] = None  # task_id
    swarm_id: Optional[str] = None
    created_at: datetime = Field(default_factory=datetime.utcnow)
    activated_at: Optional[datetime] = None
    deactivated_at: Optional[datetime] = None
    missions_completed: int = 0
    performance_score: float = 0.0
    metadata: Dict[str, Any] = Field(default_factory=dict)
