"""
Rational Reserve (RR) - Hierarchical Swarm Agent System

Line authority: Commander → Orchestrator → Director
Specialists: freelance advisors to Directors (consult only; no line command)

Lifecycle: Idle → Briefed → Executing → Reporting → Idle
"""

from .models import (
    Mission, Task, Order, SitRep, CasRep, AAR, Swarm, Unit, Assignment, AgentRecord,
    RoleEnum, RankEnum, MOSEnum, StatusEnum, PriorityEnum,
    VALID_COMMAND_TARGETS, LIFECYCLE_TRANSITIONS, ROLE_LINE_LEVEL,
)
from .rr_agent import RRAgent
from .agent_factory import AgentFactory

try:
    from .swarm_orchestrator import SwarmOrchestrator
except ImportError:
    SwarmOrchestrator = None  # type: ignore

try:
    from .c2_router import C2Router
except ImportError:
    C2Router = None  # type: ignore

try:
    from .general_mind import GeneralMind, StrategicMind
except ImportError:
    GeneralMind = None  # type: ignore
    StrategicMind = None  # type: ignore

__all__ = [
    "Mission", "Task", "Order", "SitRep", "CasRep", "AAR", "Swarm", "Unit",
    "Assignment", "AgentRecord",
    "RoleEnum", "RankEnum", "MOSEnum", "StatusEnum", "PriorityEnum",
    "VALID_COMMAND_TARGETS", "LIFECYCLE_TRANSITIONS", "ROLE_LINE_LEVEL",
    "RRAgent", "AgentFactory", "SwarmOrchestrator", "C2Router",
    "GeneralMind", "StrategicMind",
]
