"""
Agent Factory - Spawn RR agents with appropriate rank and MOS
"""

from typing import Dict, Any, Optional, List
from loguru import logger
import uuid

from .rr_agent import RRAgent
from .models import RankEnum, MOSEnum, AgentRecord


class AgentFactory:
    """
    Factory for creating Rational Reserve agents

    Manages agent lifecycle, naming, and configuration
    """

    def __init__(self, llm_clients: Dict[str, Any]):
        self.llm_clients = llm_clients
        self.active_agents: Dict[str, RRAgent] = {}
        self.agent_counter = 0
        logger.info("AgentFactory initialized")

    def spawn_agent(
        self,
        rank: RankEnum,
        mos: MOSEnum,
        name: Optional[str] = None,
        commander_id: Optional[str] = None,
        unit_id: Optional[str] = None,
    ) -> RRAgent:
        """
        Spawn a new RR agent

        Args:
            rank: Military rank
            mos: Military Occupational Specialty
            name: Agent name (auto-generated if not provided)
            commander_id: Superior officer's agent_id
            unit_id: Unit assignment

        Returns:
            Initialized RRAgent instance
        """
        agent_id = str(uuid.uuid4())

        # Generate name if not provided
        if name is None:
            name = self._generate_agent_name(rank, mos)

        # Create agent
        agent = RRAgent(
            agent_id=agent_id,
            name=name,
            rank=rank,
            mos=mos,
            llm_clients=self.llm_clients,
            commander_id=commander_id,
            unit_id=unit_id,
        )

        # Track active agent
        self.active_agents[agent_id] = agent

        logger.info(
            f"AgentFactory spawned: {name} [{rank.value}] [{mos.value}] (ID: {agent_id})"
        )

        return agent

    def spawn_general(self, name: Optional[str] = None) -> RRAgent:
        """Spawn a General - swarm commander"""
        return self.spawn_agent(
            rank=RankEnum.GENERAL,
            mos=MOSEnum.SF_ENGINEER,  # Generals are typically SF-qualified
            name=name or "General Commander",
        )

    def spawn_officer(
        self,
        rank: RankEnum,
        mos: MOSEnum,
        commander_id: Optional[str] = None,
        name: Optional[str] = None,
    ) -> RRAgent:
        """Spawn an officer (COL, MAJ, CPT, LT)"""
        if rank not in [
            RankEnum.COLONEL, RankEnum.MAJOR, RankEnum.CAPTAIN, RankEnum.LIEUTENANT
        ]:
            raise ValueError(f"{rank} is not an officer rank")

        return self.spawn_agent(
            rank=rank,
            mos=mos,
            commander_id=commander_id,
            name=name,
        )

    def spawn_nco(
        self,
        rank: RankEnum,
        mos: MOSEnum,
        commander_id: Optional[str] = None,
        name: Optional[str] = None,
    ) -> RRAgent:
        """Spawn an NCO (SGM, MSG, SGT, CPL)"""
        if rank not in [
            RankEnum.SERGEANT_MAJOR, RankEnum.MASTER_SERGEANT,
            RankEnum.SERGEANT, RankEnum.CORPORAL
        ]:
            raise ValueError(f"{rank} is not an NCO rank")

        return self.spawn_agent(
            rank=rank,
            mos=mos,
            commander_id=commander_id,
            name=name,
        )

    def spawn_specialist(
        self,
        mos: MOSEnum,
        commander_id: Optional[str] = None,
        name: Optional[str] = None,
    ) -> RRAgent:
        """Spawn a Specialist (SPC)"""
        return self.spawn_agent(
            rank=RankEnum.SPECIALIST,
            mos=mos,
            commander_id=commander_id,
            name=name,
        )

    def spawn_fire_team(
        self,
        mos: MOSEnum,
        leader_rank: RankEnum = RankEnum.CORPORAL,
        team_size: int = 4,
        commander_id: Optional[str] = None,
    ) -> List[RRAgent]:
        """
        Spawn a fire team (1 leader + specialists)

        Args:
            mos: MOS for all team members
            leader_rank: Rank of team leader (CPL or SGT)
            team_size: Total team size including leader
            commander_id: Superior officer

        Returns:
            List of agents [leader, specialist1, specialist2, ...]
        """
        if leader_rank not in [RankEnum.CORPORAL, RankEnum.SERGEANT]:
            raise ValueError("Fire team leader must be CPL or SGT")

        agents = []

        # Spawn team leader
        leader = self.spawn_agent(
            rank=leader_rank,
            mos=mos,
            commander_id=commander_id,
            name=f"{mos.name.title()} Team Leader",
        )
        agents.append(leader)

        # Spawn team members
        for i in range(team_size - 1):
            specialist = self.spawn_agent(
                rank=RankEnum.SPECIALIST,
                mos=mos,
                commander_id=leader.agent_id,
                name=f"{mos.name.title()} Specialist {i+1}",
            )
            leader.add_subordinate(specialist.agent_id)
            agents.append(specialist)

        logger.info(
            f"Fire team spawned: {team_size} agents, MOS {mos.value}, "
            f"Leader: {leader.name}"
        )

        return agents

    def spawn_squad(
        self,
        primary_mos: MOSEnum,
        support_mos: Optional[MOSEnum] = None,
        commander_id: Optional[str] = None,
    ) -> List[RRAgent]:
        """
        Spawn a squad (1 SGT + 2 fire teams)

        Args:
            primary_mos: MOS for primary fire team
            support_mos: MOS for support fire team (defaults to primary)
            commander_id: Superior officer (LT or CPT)

        Returns:
            List of agents [squad_leader, team1_leader, team1_members..., team2_leader, team2_members...]
        """
        if support_mos is None:
            support_mos = primary_mos

        agents = []

        # Spawn squad leader
        squad_leader = self.spawn_agent(
            rank=RankEnum.SERGEANT,
            mos=primary_mos,
            commander_id=commander_id,
            name=f"{primary_mos.name.title()} Squad Leader",
        )
        agents.append(squad_leader)

        # Spawn primary fire team
        team1 = self.spawn_fire_team(
            mos=primary_mos,
            leader_rank=RankEnum.CORPORAL,
            team_size=3,
            commander_id=squad_leader.agent_id,
        )
        squad_leader.add_subordinate(team1[0].agent_id)  # Add team leader
        agents.extend(team1)

        # Spawn support fire team
        team2 = self.spawn_fire_team(
            mos=support_mos,
            leader_rank=RankEnum.CORPORAL,
            team_size=3,
            commander_id=squad_leader.agent_id,
        )
        squad_leader.add_subordinate(team2[0].agent_id)  # Add team leader
        agents.extend(team2)

        logger.info(
            f"Squad spawned: {len(agents)} agents, "
            f"Primary MOS {primary_mos.value}, Support MOS {support_mos.value}"
        )

        return agents

    def spawn_platoon(
        self,
        primary_mos: MOSEnum,
        support_mos_list: Optional[List[MOSEnum]] = None,
        commander_id: Optional[str] = None,
    ) -> List[RRAgent]:
        """
        Spawn a platoon (1 LT + 3 squads)

        Args:
            primary_mos: Primary MOS for the platoon
            support_mos_list: List of support MOS (up to 2)
            commander_id: Superior officer (CPT)

        Returns:
            List of all agents in platoon
        """
        if support_mos_list is None:
            support_mos_list = [primary_mos, primary_mos]
        elif len(support_mos_list) < 2:
            support_mos_list = support_mos_list + [primary_mos] * (2 - len(support_mos_list))

        agents = []

        # Spawn platoon leader
        platoon_leader = self.spawn_agent(
            rank=RankEnum.LIEUTENANT,
            mos=primary_mos,
            commander_id=commander_id,
            name=f"{primary_mos.name.title()} Platoon Leader",
        )
        agents.append(platoon_leader)

        # Spawn 3 squads
        for i, mos in enumerate([primary_mos] + support_mos_list[:2]):
            squad = self.spawn_squad(
                primary_mos=mos,
                commander_id=platoon_leader.agent_id,
            )
            platoon_leader.add_subordinate(squad[0].agent_id)  # Add squad leader
            agents.extend(squad)

        logger.info(
            f"Platoon spawned: {len(agents)} agents, "
            f"Primary MOS {primary_mos.value}"
        )

        return agents

    def get_agent(self, agent_id: str) -> Optional[RRAgent]:
        """Get agent by ID"""
        return self.active_agents.get(agent_id)

    def deactivate_agent(self, agent_id: str):
        """Deactivate and remove agent from active pool"""
        if agent_id in self.active_agents:
            agent = self.active_agents[agent_id]
            logger.info(
                f"Deactivating agent: {agent.name} [{agent.rank.value}] (ID: {agent_id})"
            )
            del self.active_agents[agent_id]

    def get_active_agents(self) -> List[RRAgent]:
        """Get all active agents"""
        return list(self.active_agents.values())

    def get_agents_by_rank(self, rank: RankEnum) -> List[RRAgent]:
        """Get all active agents of a specific rank"""
        return [agent for agent in self.active_agents.values() if agent.rank == rank]

    def get_agents_by_mos(self, mos: MOSEnum) -> List[RRAgent]:
        """Get all active agents with a specific MOS"""
        return [agent for agent in self.active_agents.values() if agent.mos == mos]

    def get_available_agents(self) -> List[RRAgent]:
        """Get agents on standby (available for tasking)"""
        return [
            agent for agent in self.active_agents.values()
            if agent.status == "standby"
        ]

    def _generate_agent_name(self, rank: RankEnum, mos: MOSEnum) -> str:
        """Generate agent name from rank and MOS"""
        self.agent_counter += 1
        mos_name = mos.name.replace("_", " ").title()
        return f"{rank.value} {mos_name} #{self.agent_counter}"

    def get_stats(self) -> Dict[str, Any]:
        """Get factory statistics"""
        return {
            "total_active": len(self.active_agents),
            "by_rank": {
                rank.value: len(self.get_agents_by_rank(rank))
                for rank in RankEnum
            },
            "by_status": {
                "standby": len([a for a in self.active_agents.values() if a.status == "standby"]),
                "active": len([a for a in self.active_agents.values() if a.status == "active"]),
                "engaged": len([a for a in self.active_agents.values() if a.status == "engaged"]),
            },
            "agents_spawned": self.agent_counter,
        }
