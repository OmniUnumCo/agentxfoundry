"""
C2 Router - Command and Control communication routing
"""

from typing import Dict, List, Optional
from loguru import logger
from datetime import datetime

from .models import Order, SitRep, CasRep, RoleEnum, VALID_COMMAND_TARGETS
from .agent_factory import AgentFactory


class C2Router:
    """
    Command & Control Router

    Manages communication flow through the chain of command:
    - Routes orders from superiors to subordinates
    - Routes SITREPs from subordinates to superiors
    - Handles CASREPs (casualty reports)
    - Validates chain of command
    """

    def __init__(self, agent_factory: AgentFactory):
        self.agent_factory = agent_factory
        self.order_log: List[Order] = []
        self.sitrep_log: List[SitRep] = []
        self.casrep_log: List[CasRep] = []
        logger.info("C2Router initialized")

    def route_order(self, order: Order) -> bool:
        """
        Route order from superior to subordinate

        Validates chain of command before routing

        Args:
            order: Order to route

        Returns:
            True if order successfully routed
        """
        # Get agents
        from_agent = self.agent_factory.get_agent(order.from_agent)
        to_agent = self.agent_factory.get_agent(order.to_agent)

        if not from_agent or not to_agent:
            logger.error(
                f"C2Router: Order routing failed - agent not found "
                f"(from: {order.from_agent}, to: {order.to_agent})"
            )
            return False

        # Validate chain of command
        if not self._validate_chain_of_command(from_agent.agent_id, to_agent.agent_id):
            logger.warning(
                f"C2Router: Order rejected - invalid chain of command "
                f"({from_agent.name} -> {to_agent.name})"
            )
            return False

        # Route order (sender role drives recipient-side validation)
        success = to_agent.receive_order(order, sender_role=from_agent.role)

        if success:
            self.order_log.append(order)
            logger.info(
                f"C2Router: Order routed successfully "
                f"({from_agent.name} -> {to_agent.name})"
            )
        else:
            logger.warning(
                f"C2Router: Order rejected by recipient "
                f"({from_agent.name} -> {to_agent.name})"
            )

        return success

    def route_sitrep(self, sitrep: SitRep) -> bool:
        """
        Route SITREP from subordinate to superior

        Args:
            sitrep: Situation report

        Returns:
            True if SITREP successfully routed
        """
        # Get reporting agent
        from_agent = self.agent_factory.get_agent(sitrep.from_agent)

        if not from_agent:
            logger.error(f"C2Router: SITREP routing failed - agent not found")
            return False

        # If to_agent is None, it's a broadcast (goes to commander)
        if sitrep.to_agent is None:
            sitrep.to_agent = from_agent.commander_id

        # Get receiving agent
        to_agent = self.agent_factory.get_agent(sitrep.to_agent) if sitrep.to_agent else None

        if not to_agent:
            logger.warning(
                f"C2Router: SITREP has no recipient - storing for record "
                f"(from {from_agent.name})"
            )
            self.sitrep_log.append(sitrep)
            return True

        # Store SITREP
        self.sitrep_log.append(sitrep)

        logger.info(
            f"C2Router: SITREP routed "
            f"({from_agent.name} -> {to_agent.name})"
        )

        return True

    def route_casrep(self, casrep: CasRep) -> bool:
        """
        Route CASREP (casualty report) up chain of command

        CASREPs are critical and escalate to the highest available officer

        Args:
            casrep: Casualty report

        Returns:
            True if CASREP successfully routed
        """
        # Get failed agent
        agent = self.agent_factory.get_agent(casrep.agent_id)

        if not agent:
            logger.error(f"C2Router: CASREP routing failed - agent not found")
            return False

        # Store CASREP
        self.casrep_log.append(casrep)

        # Escalate to commander
        commander = self.agent_factory.get_agent(agent.commander_id) if agent.commander_id else None

        if commander:
            logger.warning(
                f"C2Router: CASREP escalated to {commander.name} - "
                f"Agent {agent.name} failed: {casrep.error_message}"
            )
        else:
            logger.warning(
                f"C2Router: CASREP logged - "
                f"Agent {agent.name} failed: {casrep.error_message}"
            )

        return True

    def broadcast_order(self, from_agent_id: str, order_content: str, order_type: str = "broadcast") -> List[Order]:
        """
        Broadcast order to all subordinates

        Args:
            from_agent_id: Commanding agent ID
            order_content: Order content
            order_type: Type of order

        Returns:
            List of orders created
        """
        from_agent = self.agent_factory.get_agent(from_agent_id)

        if not from_agent:
            logger.error(f"C2Router: Broadcast failed - agent not found")
            return []

        orders = []

        for subordinate_id in from_agent.subordinates:
            order = Order(
                from_agent=from_agent_id,
                to_agent=subordinate_id,
                order_type=order_type,
                content=order_content,
            )

            if self.route_order(order):
                orders.append(order)

        logger.info(
            f"C2Router: Broadcast from {from_agent.name} to "
            f"{len(orders)} subordinates"
        )

        return orders

    def collect_sitreps(self, commander_id: str) -> List[SitRep]:
        """
        Collect all SITREPs for a commander

        Args:
            commander_id: Commander agent ID

        Returns:
            List of SITREPs addressed to commander
        """
        sitreps = [
            sitrep for sitrep in self.sitrep_log
            if sitrep.to_agent == commander_id
        ]

        logger.info(f"C2Router: Collected {len(sitreps)} SITREPs for {commander_id}")

        return sitreps

    def get_recent_sitreps(self, agent_id: str, limit: int = 10) -> List[SitRep]:
        """
        Get recent SITREPs from an agent

        Args:
            agent_id: Agent ID
            limit: Maximum number of SITREPs

        Returns:
            List of recent SITREPs
        """
        sitreps = [
            sitrep for sitrep in self.sitrep_log
            if sitrep.from_agent == agent_id
        ]

        # Sort by timestamp descending
        sitreps.sort(key=lambda x: x.reported_at, reverse=True)

        return sitreps[:limit]

    def get_casreps(self, since: Optional[datetime] = None) -> List[CasRep]:
        """
        Get casualty reports

        Args:
            since: Only get CASREPs after this timestamp

        Returns:
            List of CASREPs
        """
        if since:
            casreps = [
                casrep for casrep in self.casrep_log
                if casrep.reported_at > since
            ]
        else:
            casreps = self.casrep_log

        return casreps

    def get_order_history(self, agent_id: str) -> List[Order]:
        """
        Get order history for an agent (sent or received)

        Args:
            agent_id: Agent ID

        Returns:
            List of orders involving agent
        """
        orders = [
            order for order in self.order_log
            if order.from_agent == agent_id or order.to_agent == agent_id
        ]

        return orders

    def _validate_chain_of_command(self, superior_id: str, subordinate_id: str) -> bool:
        """
        Validate that superior can command subordinate

        Args:
            superior_id: Superior agent ID
            subordinate_id: Subordinate agent ID

        Returns:
            True if chain of command is valid
        """
        superior = self.agent_factory.get_agent(superior_id)
        subordinate = self.agent_factory.get_agent(subordinate_id)

        if not superior or not subordinate:
            return False

        # RoleEnum is authoritative: no rank-based overrides or skip-level orders.
        if subordinate.commander_id != superior_id and subordinate_id not in superior.subordinates:
            return False
        if superior.role == RoleEnum.DIRECTOR:
            # Directors task their own sub-agents (non-Specialist, non-SpecOps)
            return subordinate.role not in (RoleEnum.SPECIALIST, RoleEnum.SPECOPS)
        return subordinate.role in VALID_COMMAND_TARGETS.get(superior.role, set())

    def get_communication_stats(self) -> Dict[str, int]:
        """Get communication statistics"""
        return {
            "total_orders": len(self.order_log),
            "total_sitreps": len(self.sitrep_log),
            "total_casreps": len(self.casrep_log),
            "acknowledged_orders": len([o for o in self.order_log if o.acknowledged_at]),
            "recent_casreps": len([c for c in self.casrep_log if (datetime.utcnow() - c.reported_at).seconds < 3600]),
        }

    def clear_logs(self, older_than: Optional[datetime] = None):
        """
        Clear old communication logs

        Args:
            older_than: Clear logs older than this timestamp
        """
        if older_than:
            self.order_log = [o for o in self.order_log if o.issued_at > older_than]
            self.sitrep_log = [s for s in self.sitrep_log if s.reported_at > older_than]
            self.casrep_log = [c for c in self.casrep_log if c.reported_at > older_than]
            logger.info(f"C2Router: Cleared logs older than {older_than}")
        else:
            self.order_log.clear()
            self.sitrep_log.clear()
            self.casrep_log.clear()
            logger.info("C2Router: All logs cleared")
