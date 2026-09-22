"""
Agent Factory - Spawn RR agents by RoleEnum

Line authority: Commander → Orchestrator → Director
Specialists: freelance advisors (not line subordinates above Directors)

spawn_general is deprecated — wraps spawn_orchestrator (or spawn_commander
when name/context implies top-level command).
"""

from typing import Dict, Any, Optional, List
from loguru import logger
import uuid
import warnings

from .rr_agent import RRAgent
from .models import RoleEnum, RankEnum, MOSEnum, StatusEnum, AgentRecord


class AgentFactory:
    """Factory for creating Rational Reserve agents by RoleEnum."""

    def __init__(self, llm_clients: Dict[str, Any]):
        self.llm_clients = llm_clients
        self.active_agents: Dict[str, RRAgent] = {}
        self.agent_counter = 0
        logger.info("AgentFactory initialized")

    def spawn_agent(
        self,
        role: RoleEnum,
        name: Optional[str] = None,
        commander_id: Optional[str] = None,
        unit_id: Optional[str] = None,
        rank: Optional[RankEnum] = None,
        mos: Optional[MOSEnum] = None,
    ) -> RRAgent:
        """Spawn a new RR agent with primary RoleEnum."""
        agent_id = str(uuid.uuid4())
        if name is None:
            name = self._generate_agent_name(role, mos)

        agent = RRAgent(
            agent_id=agent_id,
            name=name,
            role=role,
            llm_clients=self.llm_clients,
            commander_id=commander_id,
            unit_id=unit_id,
            rank=rank,
            mos=mos,
        )
        self.active_agents[agent_id] = agent
        logger.info(
            f"AgentFactory spawned: {name} [role={role.value}] "
            f"(ID: {agent_id})"
        )
        return agent

    def spawn_commander(self, name: Optional[str] = None) -> RRAgent:
        """Spawn a Commander — optional top of hierarchy (no commander_id)."""
        return self.spawn_agent(
            role=RoleEnum.COMMANDER,
            name=name or "Commander",
            mos=MOSEnum.SF_ENGINEER,
        )

    def spawn_orchestrator(
        self,
        name: Optional[str] = None,
        commander_id: Optional[str] = None,
    ) -> RRAgent:
        """Spawn an Orchestrator — mission planner under optional Commander."""
        return self.spawn_agent(
            role=RoleEnum.ORCHESTRATOR,
            name=name or "Orchestrator",
            commander_id=commander_id,
            mos=MOSEnum.SF_ENGINEER,
        )

    def spawn_director(
        self,
        name: Optional[str] = None,
        commander_id: Optional[str] = None,
        mos: Optional[MOSEnum] = None,
        rank: Optional[RankEnum] = None,
    ) -> RRAgent:
        """Spawn a Director — owns a workstream under Orchestrator."""
        return self.spawn_agent(
            role=RoleEnum.DIRECTOR,
            name=name or "Director",
            commander_id=commander_id,
            mos=mos or MOSEnum.COMBAT_ENGINEER,
            rank=rank or RankEnum.COLONEL,
        )

    def spawn_specialist(
        self,
        mos: Optional[MOSEnum] = None,
        name: Optional[str] = None,
        commander_id: Optional[str] = None,
        advising: Optional[str] = None,
    ) -> RRAgent:
        """
        Spawn a Specialist — freelance advisor (not line command over Directors).

        commander_id may be the Orchestrator who engaged them.
        advising is the Director agent_id they advise (optional).
        """
        agent = self.spawn_agent(
            role=RoleEnum.SPECIALIST,
            name=name,
            commander_id=commander_id,
            mos=mos or MOSEnum.INFANTRY,
            rank=RankEnum.SPECIALIST,
        )
        if advising:
            agent.advising = advising
        return agent

    def spawn_general(self, name: Optional[str] = None) -> RRAgent:
        """
        DEPRECATED: use spawn_orchestrator() or spawn_commander().

        Historically "General" was the swarm commander. Under the Jason hierarchy
        that maps to Orchestrator (mission planning/delegation). If you need the
        optional top-level role, call spawn_commander() instead.

        This wrapper calls spawn_orchestrator() for backward compatibility.
        """
        warnings.warn(
            "spawn_general is deprecated; use spawn_orchestrator() "
            "(or spawn_commander() for top-level). "
            "Maps legacy General → Orchestrator.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.spawn_orchestrator(name=name or "General Commander")

    def spawn_officer(
        self,
        rank: RankEnum,
        mos: MOSEnum,
        commander_id: Optional[str] = None,
        name: Optional[str] = None,
    ) -> RRAgent:
        """Legacy helper: spawn Director with officer rank metadata."""
        if rank not in [
            RankEnum.COLONEL, RankEnum.MAJOR, RankEnum.CAPTAIN, RankEnum.LIEUTENANT
        ]:
            raise ValueError(f"{rank} is not an officer rank")
        return self.spawn_director(
            name=name,
            commander_id=commander_id,
            mos=mos,
            rank=rank,
        )

    def spawn_nco(
        self,
        rank: RankEnum,
        mos: MOSEnum,
        commander_id: Optional[str] = None,
        name: Optional[str] = None,
    ) -> RRAgent:
        """Legacy helper: spawn operational agent under a Director (role=DIRECTOR metadata)."""
        if rank not in [
            RankEnum.SERGEANT_MAJOR, RankEnum.MASTER_SERGEANT,
            RankEnum.SERGEANT, RankEnum.CORPORAL,
        ]:
            raise ValueError(f"{rank} is not an NCO rank")
        return self.spawn_agent(
            role=RoleEnum.DIRECTOR,
            name=name,
            commander_id=commander_id,
            mos=mos,
            rank=rank,
        )

    def spawn_workstream(
        self,
        orchestrator_id: str,
        primary_mos: MOSEnum = MOSEnum.COMBAT_ENGINEER,
        specialist_count: int = 1,
        director_name: Optional[str] = None,
        num_specialists: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Spawn one workstream: 1 Director (+ 0–2 Specialist advisors).

        Caps: max 1 Director + 1–2 Specialists per workstream.
        Orchestrator assigns the Director; Specialists attach as advisors.
        """
        if num_specialists is not None:
            specialist_count = num_specialists
        if specialist_count < 0 or specialist_count > 2:
            raise ValueError("specialist_count must be 0–2 per workstream")

        director = self.spawn_director(
            name=director_name or f"{primary_mos.name.title()} Director",
            commander_id=orchestrator_id,
            mos=primary_mos,
        )
        orch = self.get_agent(orchestrator_id)
        if orch:
            orch.add_subordinate(director.agent_id)

        specialists = []
        for i in range(specialist_count):
            spec = self.spawn_specialist(
                mos=primary_mos,
                commander_id=orchestrator_id,
                advising=director.agent_id,
                name=f"{primary_mos.name.title()} Advisor {i+1}",
            )
            director.attach_advisor(spec.agent_id)
            specialists.append(spec)

        logger.info(
            f"Workstream spawned: Director={director.name}, "
            f"advisors={len(specialists)}"
        )
        return {
            "director": director,
            "specialists": specialists,
            "agents": [director] + specialists,
        }

    # Legacy formation helpers (kept; wire under Orchestrator as Directors)
    def spawn_fire_team(
        self,
        mos: MOSEnum,
        leader_rank: RankEnum = RankEnum.CORPORAL,
        team_size: int = 4,
        commander_id: Optional[str] = None,
    ) -> List[RRAgent]:
        """Legacy: spawn Director-led small team (leader + specialists as advisors/ops)."""
        agents = []
        leader = self.spawn_director(
            name=f"{mos.name.title()} Team Lead",
            commander_id=commander_id,
            mos=mos,
            rank=leader_rank if leader_rank in (
                RankEnum.COLONEL, RankEnum.MAJOR, RankEnum.CAPTAIN,
                RankEnum.LIEUTENANT, RankEnum.SERGEANT, RankEnum.CORPORAL,
            ) else RankEnum.CORPORAL,
        )
        # Re-label rank on leader for legacy
        leader.rank = leader_rank
        agents.append(leader)
        for i in range(max(0, team_size - 1)):
            member = self.spawn_specialist(
                mos=mos,
                commander_id=leader.agent_id,
                advising=leader.agent_id,
                name=f"{mos.name.title()} Specialist {i+1}",
            )
            leader.attach_advisor(member.agent_id)
            # Also track as subordinate for legacy fire-team ops
            leader.add_subordinate(member.agent_id)
            agents.append(member)
        return agents

    def spawn_squad(
        self,
        primary_mos: MOSEnum,
        support_mos: Optional[MOSEnum] = None,
        commander_id: Optional[str] = None,
    ) -> List[RRAgent]:
        if support_mos is None:
            support_mos = primary_mos
        agents = []
        squad_leader = self.spawn_director(
            name=f"{primary_mos.name.title()} Squad Lead",
            commander_id=commander_id,
            mos=primary_mos,
            rank=RankEnum.SERGEANT,
        )
        agents.append(squad_leader)
        for mos in (primary_mos, support_mos):
            team = self.spawn_fire_team(
                mos=mos, leader_rank=RankEnum.CORPORAL, team_size=3,
                commander_id=squad_leader.agent_id,
            )
            squad_leader.add_subordinate(team[0].agent_id)
            agents.extend(team)
        return agents

    def spawn_platoon(
        self,
        primary_mos: MOSEnum,
        support_mos_list: Optional[List[MOSEnum]] = None,
        commander_id: Optional[str] = None,
    ) -> List[RRAgent]:
        if support_mos_list is None:
            support_mos_list = [primary_mos, primary_mos]
        elif len(support_mos_list) < 2:
            support_mos_list = support_mos_list + [primary_mos] * (2 - len(support_mos_list))
        agents = []
        platoon_leader = self.spawn_director(
            name=f"{primary_mos.name.title()} Platoon Lead",
            commander_id=commander_id,
            mos=primary_mos,
            rank=RankEnum.LIEUTENANT,
        )
        agents.append(platoon_leader)
        for mos in [primary_mos] + support_mos_list[:2]:
            squad = self.spawn_squad(primary_mos=mos, commander_id=platoon_leader.agent_id)
            platoon_leader.add_subordinate(squad[0].agent_id)
            agents.extend(squad)
        return agents

    def get_agent(self, agent_id: str) -> Optional[RRAgent]:
        return self.active_agents.get(agent_id)

    def deactivate_agent(self, agent_id: str):
        if agent_id in self.active_agents:
            agent = self.active_agents[agent_id]
            logger.info(f"Deactivating agent: {agent.name} [{agent.role.value}]")
            del self.active_agents[agent_id]

    def get_active_agents(self) -> List[RRAgent]:
        return list(self.active_agents.values())

    def get_agents_by_role(self, role: RoleEnum) -> List[RRAgent]:
        return [a for a in self.active_agents.values() if a.role == role]

    def get_agents_by_rank(self, rank: RankEnum) -> List[RRAgent]:
        return [a for a in self.active_agents.values() if a.rank == rank]

    def get_agents_by_mos(self, mos: MOSEnum) -> List[RRAgent]:
        return [a for a in self.active_agents.values() if a.mos == mos]

    def get_available_agents(self) -> List[RRAgent]:
        return [
            a for a in self.active_agents.values()
            if a.status == StatusEnum.IDLE
        ]

    def _generate_agent_name(self, role: RoleEnum, mos: Optional[MOSEnum]) -> str:
        self.agent_counter += 1
        mos_part = f" {mos.name.replace('_', ' ').title()}" if mos else ""
        return f"{role.value.replace('_', ' ').title()}{mos_part} #{self.agent_counter}"

    def get_stats(self) -> Dict[str, Any]:
        return {
            "total_active": len(self.active_agents),
            "by_role": {
                role.value: len(self.get_agents_by_role(role))
                for role in RoleEnum
            },
            "by_rank": {
                rank.value: len(self.get_agents_by_rank(rank))
                for rank in RankEnum
            },
            "by_status": {
                "idle": len([a for a in self.active_agents.values() if a.status == StatusEnum.IDLE]),
                "briefed": len([a for a in self.active_agents.values() if a.status == StatusEnum.BRIEFED]),
                "executing": len([a for a in self.active_agents.values() if a.status == StatusEnum.EXECUTING]),
                "reporting": len([a for a in self.active_agents.values() if a.status == StatusEnum.REPORTING]),
            },
            "agents_spawned": self.agent_counter,
        }
