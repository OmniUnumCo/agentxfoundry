"""
Deterministic unit tests for Jason hierarchy + lifecycle.
No live LLM calls.

Line authority: Commander → Orchestrator → Director
Specialists: freelance advisors (consult only; do not command Directors)
Lifecycle: Idle → Briefed → Executing → Reporting → Idle
"""

from __future__ import annotations

import sys
import types
import unittest
import warnings
from pathlib import Path
from unittest.mock import MagicMock

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

try:
    from utils.base_agent import BaseAgent  # noqa: F401
except Exception:
    utils_pkg = types.ModuleType("utils")
    base_mod = types.ModuleType("utils.base_agent")

    class BaseAgent:
        def __init__(self, name, llm_clients):
            self.name = name
            self.llm_clients = llm_clients

        def query_llm(self, llm_name, prompt):
            return f"mock:{prompt[:40]}"

        def process(self, input_data):
            raise NotImplementedError

    base_mod.BaseAgent = BaseAgent
    sys.modules["utils"] = utils_pkg
    sys.modules["utils.base_agent"] = base_mod

from rr.models import (
    RoleEnum,
    StatusEnum,
    Order,
    Assignment,
    VALID_COMMAND_TARGETS,
    LIFECYCLE_TRANSITIONS,
)
from rr.agent_factory import AgentFactory


def _factory() -> AgentFactory:
    return AgentFactory(llm_clients={"anthropic": MagicMock(), "llama": MagicMock()})


class TestChainOfCommand(unittest.TestCase):
    def test_valid_command_targets(self):
        self.assertEqual(
            VALID_COMMAND_TARGETS[RoleEnum.COMMANDER],
            {RoleEnum.ORCHESTRATOR},
        )
        self.assertEqual(
            VALID_COMMAND_TARGETS[RoleEnum.ORCHESTRATOR],
            {RoleEnum.DIRECTOR},
        )
        self.assertEqual(VALID_COMMAND_TARGETS[RoleEnum.DIRECTOR], set())
        self.assertEqual(VALID_COMMAND_TARGETS[RoleEnum.SPECIALIST], set())
        self.assertNotIn(RoleEnum.DIRECTOR, VALID_COMMAND_TARGETS[RoleEnum.SPECIALIST])
        self.assertNotIn(RoleEnum.SPECIALIST, VALID_COMMAND_TARGETS[RoleEnum.ORCHESTRATOR])

    def test_orchestrator_assigns_director(self):
        f = _factory()
        orch = f.spawn_orchestrator(name="Orch")
        director = f.spawn_director(name="Dir", commander_id=orch.agent_id)
        orch.add_subordinate(director.agent_id)

        order = Order(
            from_agent=orch.agent_id,
            to_agent=director.agent_id,
            order_type="assignment",
            content="Own auth workstream",
            goal="Ship auth",
            scope_boundary="API only",
            done_criteria=["tests green"],
            stop_list=["no prod deploys"],
            role_name="director",
        )
        ok = director.receive_order(order, sender_role=RoleEnum.ORCHESTRATOR)
        self.assertTrue(ok)
        self.assertEqual(director.status, StatusEnum.BRIEFED)
        self.assertIsNotNone(director.current_assignment)
        self.assertEqual(director.current_assignment.goal, "Ship auth")
        self.assertEqual(director.current_assignment.scope_boundary, "API only")
        self.assertEqual(director.current_assignment.done_criteria, ["tests green"])
        self.assertEqual(director.current_assignment.stop_list, ["no prod deploys"])

    def test_director_rejects_specialist_command(self):
        f = _factory()
        orch = f.spawn_orchestrator()
        director = f.spawn_director(commander_id=orch.agent_id)
        specialist = f.spawn_specialist(
            commander_id=orch.agent_id, advising=director.agent_id
        )

        order = Order(
            from_agent=specialist.agent_id,
            to_agent=director.agent_id,
            order_type="assignment",
            content="Do this",
            is_advisory=False,
        )
        ok = director.receive_order(order, sender_role=RoleEnum.SPECIALIST)
        self.assertFalse(ok)
        self.assertEqual(director.status, StatusEnum.IDLE)

    def test_specialist_accepts_advisory_brief(self):
        f = _factory()
        orch = f.spawn_orchestrator()
        director = f.spawn_director(commander_id=orch.agent_id)
        specialist = f.spawn_specialist(
            commander_id=orch.agent_id, advising=director.agent_id
        )
        director.attach_advisor(specialist.agent_id)

        brief = Order(
            from_agent=director.agent_id,
            to_agent=specialist.agent_id,
            order_type="advisory_brief",
            content="Advise on crypto choices",
            goal="Recommend library",
            is_advisory=True,
            role_name="specialist",
        )
        ok = specialist.receive_order(brief, sender_role=RoleEnum.DIRECTOR)
        self.assertTrue(ok)
        self.assertEqual(specialist.status, StatusEnum.BRIEFED)

    def test_specialist_rejects_peer_assign(self):
        f = _factory()
        orch = f.spawn_orchestrator()
        s1 = f.spawn_specialist(name="S1", commander_id=orch.agent_id)
        s2 = f.spawn_specialist(name="S2", commander_id=orch.agent_id)

        order = Order(
            from_agent=s1.agent_id,
            to_agent=s2.agent_id,
            order_type="assignment",
            content="Reassign you",
        )
        ok = s2.receive_order(order, sender_role=RoleEnum.SPECIALIST)
        self.assertFalse(ok)

    def test_skip_level_command_rejected(self):
        f = _factory()
        cmd = f.spawn_commander()
        orch = f.spawn_orchestrator(commander_id=cmd.agent_id)
        director = f.spawn_director(commander_id=orch.agent_id)

        order = Order(
            from_agent=cmd.agent_id,
            to_agent=director.agent_id,
            order_type="assignment",
            content="Skip level",
        )
        ok = director.receive_order(order, sender_role=RoleEnum.COMMANDER)
        self.assertFalse(ok)

    def test_commander_assigns_orchestrator(self):
        f = _factory()
        cmd = f.spawn_commander()
        orch = f.spawn_orchestrator(commander_id=cmd.agent_id)
        cmd.add_subordinate(orch.agent_id)

        order = Order(
            from_agent=cmd.agent_id,
            to_agent=orch.agent_id,
            order_type="assignment",
            content="Run mission",
            goal="Mission X",
            role_name="orchestrator",
        )
        self.assertTrue(orch.receive_order(order, sender_role=RoleEnum.COMMANDER))
        self.assertEqual(orch.status, StatusEnum.BRIEFED)


class TestLifecycle(unittest.TestCase):
    def test_idle_briefed_executing_reporting_idle(self):
        f = _factory()
        orch = f.spawn_orchestrator()
        director = f.spawn_director(commander_id=orch.agent_id)
        self.assertEqual(director.status, StatusEnum.IDLE)

        order = Order(
            from_agent=orch.agent_id,
            to_agent=director.agent_id,
            order_type="assignment",
            content="Build feature",
            goal="Build feature",
        )
        self.assertTrue(director.receive_order(order, sender_role=RoleEnum.ORCHESTRATOR))
        self.assertEqual(director.status, StatusEnum.BRIEFED)

        self.assertTrue(director.transition_status(StatusEnum.EXECUTING))
        self.assertEqual(director.status, StatusEnum.EXECUTING)

        self.assertTrue(director.transition_status(StatusEnum.REPORTING))
        sitrep = director.report_sitrep(
            kind="done",
            done=["feature built"],
            next_actions=[],
            message="Done",
        )
        self.assertEqual(sitrep.report_kind, "done")
        self.assertEqual(sitrep.done, ["feature built"])
        self.assertEqual(director.status, StatusEnum.IDLE)

    def test_illegal_transition_rejected(self):
        f = _factory()
        d = f.spawn_director()
        self.assertEqual(d.status, StatusEnum.IDLE)
        self.assertFalse(d.transition_status(StatusEnum.REPORTING))
        self.assertEqual(d.status, StatusEnum.IDLE)

    def test_blocked_sitrep(self):
        f = _factory()
        orch = f.spawn_orchestrator()
        d = f.spawn_director(commander_id=orch.agent_id)
        d.receive_order(
            Order(
                from_agent=orch.agent_id,
                to_agent=d.agent_id,
                order_type="assignment",
                content="Work",
            ),
            sender_role=RoleEnum.ORCHESTRATOR,
        )
        d.transition_status(StatusEnum.EXECUTING)
        sitrep = d.report_sitrep(
            kind="blocked", blockers=["waiting on key"], message="Blocked"
        )
        self.assertEqual(sitrep.report_kind, "blocked")
        self.assertEqual(d.status, StatusEnum.BLOCKED)
        self.assertIn("waiting on key", sitrep.blockers)

    def test_lifecycle_transition_map(self):
        self.assertIn(StatusEnum.BRIEFED, LIFECYCLE_TRANSITIONS[StatusEnum.IDLE])
        self.assertIn(StatusEnum.EXECUTING, LIFECYCLE_TRANSITIONS[StatusEnum.BRIEFED])
        self.assertIn(StatusEnum.REPORTING, LIFECYCLE_TRANSITIONS[StatusEnum.EXECUTING])
        self.assertIn(StatusEnum.IDLE, LIFECYCLE_TRANSITIONS[StatusEnum.REPORTING])


class TestFactoryHierarchy(unittest.TestCase):
    def test_spawn_roles(self):
        f = _factory()
        c = f.spawn_commander()
        o = f.spawn_orchestrator(commander_id=c.agent_id)
        d = f.spawn_director(commander_id=o.agent_id)
        s = f.spawn_specialist(commander_id=o.agent_id, advising=d.agent_id)
        self.assertEqual(c.role, RoleEnum.COMMANDER)
        self.assertEqual(o.role, RoleEnum.ORCHESTRATOR)
        self.assertEqual(d.role, RoleEnum.DIRECTOR)
        self.assertEqual(s.role, RoleEnum.SPECIALIST)
        self.assertEqual(s.advising, d.agent_id)

    def test_spawn_workstream_caps(self):
        f = _factory()
        o = f.spawn_orchestrator()
        ws = f.spawn_workstream(o.agent_id, specialist_count=2)
        self.assertEqual(ws["director"].role, RoleEnum.DIRECTOR)
        self.assertEqual(len(ws["specialists"]), 2)
        self.assertEqual(len(ws["director"].advisors), 2)
        with self.assertRaises(ValueError):
            f.spawn_workstream(o.agent_id, specialist_count=3)

    def test_spawn_general_deprecated_maps_to_orchestrator(self):
        f = _factory()
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            g = f.spawn_general("Legacy Gen")
            self.assertTrue(any(issubclass(x.category, DeprecationWarning) for x in w))
        self.assertEqual(g.role, RoleEnum.ORCHESTRATOR)

    def test_can_command_role(self):
        f = _factory()
        o = f.spawn_orchestrator()
        d = f.spawn_director()
        s = f.spawn_specialist()
        self.assertTrue(o.can_command_role(RoleEnum.DIRECTOR))
        self.assertFalse(o.can_command_role(RoleEnum.SPECIALIST))
        self.assertFalse(s.can_command_role(RoleEnum.DIRECTOR))
        self.assertFalse(d.can_command_role(RoleEnum.SPECIALIST))


class TestAssignmentContract(unittest.TestCase):
    def test_order_to_assignment(self):
        order = Order(
            from_agent="a",
            to_agent="b",
            order_type="assignment",
            content="fallback",
            goal="G",
            scope_boundary="S",
            done_criteria=["d1"],
            stop_list=["x"],
            role_name="director",
        )
        a = order.to_assignment()
        self.assertIsInstance(a, Assignment)
        self.assertEqual(a.goal, "G")
        self.assertEqual(a.scope_boundary, "S")
        self.assertEqual(a.done_criteria, ["d1"])
        self.assertEqual(a.stop_list, ["x"])
        self.assertEqual(a.role_name, "director")


class TestStatusAliases(unittest.TestCase):
    def test_legacy_aliases_map(self):
        self.assertEqual(StatusEnum.STANDBY.value, StatusEnum.IDLE.value)
        self.assertEqual(StatusEnum.ACTIVE.value, StatusEnum.EXECUTING.value)
        self.assertEqual(StatusEnum.ENGAGED.value, StatusEnum.EXECUTING.value)
        self.assertEqual(StatusEnum.COMPLETE.value, StatusEnum.REPORTING.value)


if __name__ == "__main__":
    unittest.main()
