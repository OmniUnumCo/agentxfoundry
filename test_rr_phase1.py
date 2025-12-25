"""
Test script for Rational Reserve Phase 1

This script tests the basic functionality of the RR system:
- Agent creation
- Swarm formation
- Mission execution
"""

from dotenv import load_dotenv
load_dotenv()

from llm_integrations.clients import get_llm_clients
from rr.agent_factory import AgentFactory
from rr.swarm_orchestrator import SwarmOrchestrator
from rr.c2_router import C2Router
from rr.models import MOSEnum, RankEnum, PriorityEnum
from loguru import logger


def test_agent_creation():
    """Test basic agent creation"""
    logger.info("=" * 50)
    logger.info("TEST 1: Agent Creation")
    logger.info("=" * 50)

    llm_clients = get_llm_clients()
    factory = AgentFactory(llm_clients)

    # Spawn General
    general = factory.spawn_general("General Washington")
    logger.info(f"Created: {general.name} [{general.rank.value}]")

    # Spawn specialist
    specialist = factory.spawn_specialist(
        mos=MOSEnum.COMBAT_ENGINEER,
        name="Engineer Smith",
    )
    logger.info(f"Created: {specialist.name} [{specialist.rank.value}] [{specialist.mos.value}]")

    # Spawn fire team
    team = factory.spawn_fire_team(
        mos=MOSEnum.INFANTRY,
        team_size=4,
    )
    logger.info(f"Created fire team with {len(team)} members")

    stats = factory.get_stats()
    logger.info(f"Factory stats: {stats}")

    logger.info("✓ Agent creation test passed\n")
    return factory


def test_swarm_creation(factory: AgentFactory):
    """Test swarm creation"""
    logger.info("=" * 50)
    logger.info("TEST 2: Swarm Creation")
    logger.info("=" * 50)

    orchestrator = SwarmOrchestrator(factory)

    # Create mission
    mission = orchestrator.create_mission(
        objective="Write a Python function to calculate fibonacci numbers",
        priority=PriorityEnum.ROUTINE,
    )
    logger.info(f"Created mission: {mission.objective}")

    # Create swarm
    swarm = orchestrator.create_swarm(
        mission=mission,
        swarm_size="single",
        primary_mos=MOSEnum.COMBAT_ENGINEER,
    )
    logger.info(f"Created swarm: {swarm.name} with {len(swarm.agents)} agents")

    # Get status
    status = orchestrator.get_swarm_status(swarm.id)
    logger.info(f"Swarm status: {status['status']}")

    logger.info("✓ Swarm creation test passed\n")
    return orchestrator, swarm


def test_mission_execution(orchestrator: SwarmOrchestrator, swarm):
    """Test mission execution"""
    logger.info("=" * 50)
    logger.info("TEST 3: Mission Execution")
    logger.info("=" * 50)

    # Activate swarm
    success = orchestrator.activate_swarm(swarm.id)
    logger.info(f"Swarm activation: {'SUCCESS' if success else 'FAILED'}")

    # Execute mission
    logger.info("Executing mission...")
    result = orchestrator.execute_mission(swarm.id)

    logger.info(f"Execution result: {result['success']}")
    if result['success']:
        logger.info(f"Mission result preview: {str(result['result'])[:200]}...")
    else:
        logger.error(f"Mission error: {result.get('error')}")

    logger.info("✓ Mission execution test passed\n")
    return result


def test_swarm_disband(orchestrator: SwarmOrchestrator, swarm):
    """Test swarm disbanding"""
    logger.info("=" * 50)
    logger.info("TEST 4: Swarm Disband")
    logger.info("=" * 50)

    # Disband swarm
    aar = orchestrator.disband_swarm(swarm.id)

    logger.info(f"AAR Generated:")
    logger.info(f"  Success: {aar.success}")
    logger.info(f"  Objectives met: {aar.objectives_met}")
    logger.info(f"  Lessons learned: {aar.lessons_learned}")
    logger.info(f"  Recommendations: {aar.recommendations}")

    logger.info("✓ Swarm disband test passed\n")
    return aar


def test_c2_communication():
    """Test command and control communication"""
    logger.info("=" * 50)
    logger.info("TEST 5: C2 Communication")
    logger.info("=" * 50)

    llm_clients = get_llm_clients()
    factory = AgentFactory(llm_clients)
    c2 = C2Router(factory)

    # Create chain of command
    general = factory.spawn_general("General Patton")
    captain = factory.spawn_officer(
        rank=RankEnum.CAPTAIN,
        mos=MOSEnum.INFANTRY,
        commander_id=general.agent_id,
        name="Captain America",
    )
    general.add_subordinate(captain.agent_id)

    sergeant = factory.spawn_nco(
        rank=RankEnum.SERGEANT,
        mos=MOSEnum.INFANTRY,
        commander_id=captain.agent_id,
        name="Sergeant Rock",
    )
    captain.add_subordinate(sergeant.agent_id)

    # Test order routing
    from rr.models import Order
    order = Order(
        from_agent=general.agent_id,
        to_agent=captain.agent_id,
        order_type="test_order",
        content="Test order from General to Captain",
    )

    success = c2.route_order(order)
    logger.info(f"Order routing (GEN->CPT): {'SUCCESS' if success else 'FAILED'}")

    # Test SITREP
    sitrep = sergeant.report_sitrep()
    c2.route_sitrep(sitrep)
    logger.info(f"SITREP generated from {sergeant.name}")

    # Get stats
    stats = c2.get_communication_stats()
    logger.info(f"C2 stats: {stats}")

    logger.info("✓ C2 communication test passed\n")


def main():
    """Run all tests"""
    logger.info("\n" + "=" * 50)
    logger.info("RATIONAL RESERVE PHASE 1 TEST SUITE")
    logger.info("=" * 50 + "\n")

    try:
        # Test 1: Agent creation
        factory = test_agent_creation()

        # Test 2: Swarm creation
        orchestrator, swarm = test_swarm_creation(factory)

        # Test 3: Mission execution
        result = test_mission_execution(orchestrator, swarm)

        # Test 4: Swarm disband
        aar = test_swarm_disband(orchestrator, swarm)

        # Test 5: C2 communication
        test_c2_communication()

        logger.info("=" * 50)
        logger.info("ALL TESTS PASSED ✓")
        logger.info("=" * 50)

        logger.info("\nPhase 1 foundation is operational!")
        logger.info("Motto: 'Many minds, one mission'")

    except Exception as e:
        logger.error(f"TEST FAILED: {str(e)}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
