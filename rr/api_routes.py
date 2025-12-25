"""
FastAPI routes for Rational Reserve swarm system
"""

from fastapi import APIRouter, HTTPException, Depends
from typing import Optional
from pydantic import BaseModel

from utils.security import verify_token
from .models import PriorityEnum, MOSEnum
from .swarm_orchestrator import SwarmOrchestrator


router = APIRouter(prefix="/rr", tags=["Rational Reserve"])

# Global orchestrator instance (will be set on initialization)
orchestrator: Optional[SwarmOrchestrator] = None


def set_orchestrator(orch: SwarmOrchestrator):
    """Set the global orchestrator instance"""
    global orchestrator
    orchestrator = orch


def get_orchestrator() -> SwarmOrchestrator:
    """Dependency to get orchestrator"""
    if orchestrator is None:
        raise HTTPException(status_code=500, detail="Orchestrator not initialized")
    return orchestrator


# Request/Response models
class CreateMissionRequest(BaseModel):
    objective: str
    description: Optional[str] = None
    priority: PriorityEnum = PriorityEnum.ROUTINE
    swarm_size: str = "fire_team"  # fire_team, squad, platoon, single
    primary_mos: MOSEnum = MOSEnum.INFANTRY


class CreateSwarmResponse(BaseModel):
    swarm_id: str
    mission_id: str
    name: str
    agent_count: int
    general_name: str
    status: str


class ExecuteMissionResponse(BaseModel):
    success: bool
    mission_id: str
    swarm_id: str
    result: Optional[str] = None
    error: Optional[str] = None


# Endpoints
@router.get("/")
def rr_root():
    """Rational Reserve system info"""
    return {
        "system": "Rational Reserve (RR)",
        "motto": "Many minds, one mission",
        "status": "operational",
        "phase": "1 - Foundation",
    }


@router.post("/mission/create", response_model=CreateSwarmResponse)
def create_mission(
    request: CreateMissionRequest,
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """
    Create a new mission and spawn swarm

    This endpoint:
    1. Creates a mission from the objective
    2. Spawns a swarm with appropriate size and MOS
    3. Returns swarm information
    """
    try:
        # Create mission
        mission = orch.create_mission(
            objective=request.objective,
            description=request.description,
            priority=request.priority,
        )

        # Create swarm
        swarm = orch.create_swarm(
            mission=mission,
            swarm_size=request.swarm_size,
            primary_mos=request.primary_mos,
        )

        # Get general info
        general = orch.agent_factory.get_agent(swarm.general_id)

        return CreateSwarmResponse(
            swarm_id=swarm.id,
            mission_id=mission.id,
            name=swarm.name,
            agent_count=len(swarm.agents),
            general_name=general.name if general else "Unknown",
            status=swarm.status.value,
        )

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/swarm/{swarm_id}/execute", response_model=ExecuteMissionResponse)
def execute_swarm(
    swarm_id: str,
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """
    Activate and execute swarm mission

    This endpoint:
    1. Activates the swarm
    2. Executes the mission
    3. Returns results
    """
    try:
        # Activate swarm
        if not orch.activate_swarm(swarm_id):
            raise HTTPException(status_code=404, detail="Swarm not found or activation failed")

        # Execute mission
        result = orch.execute_mission(swarm_id)

        return ExecuteMissionResponse(**result)

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/swarm/{swarm_id}/status")
def get_swarm_status(
    swarm_id: str,
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """Get current swarm status"""
    try:
        status = orch.get_swarm_status(swarm_id)

        if "error" in status:
            raise HTTPException(status_code=404, detail=status["error"])

        return status

    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/swarm/{swarm_id}/disband")
def disband_swarm(
    swarm_id: str,
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """Disband swarm and get After Action Review"""
    try:
        aar = orch.disband_swarm(swarm_id)

        return {
            "swarm_id": swarm_id,
            "success": aar.success,
            "aar": aar.dict(),
        }

    except ValueError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/swarms")
def list_swarms(
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """List all active swarms"""
    try:
        swarms = orch.get_active_swarms()
        return {"swarms": swarms, "count": len(swarms)}

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/stats")
def get_stats(
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """Get system statistics"""
    try:
        stats = orch.get_stats()
        return stats

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/agents/roster")
def get_agent_roster(
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """Get roster of all active agents"""
    try:
        agents = orch.agent_factory.get_active_agents()

        roster = [
            {
                "id": agent.agent_id,
                "name": agent.name,
                "rank": agent.rank.value,
                "mos": agent.mos.value,
                "status": agent.status.value,
                "swarm_id": agent.swarm_id,
                "unit_id": agent.unit_id,
            }
            for agent in agents
        ]

        return {"agents": roster, "count": len(roster)}

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/mission/quick")
def quick_mission(
    objective: str,
    token: str = Depends(verify_token),
    orch: SwarmOrchestrator = Depends(get_orchestrator),
):
    """
    Quick mission execution (create + execute in one call)

    For simple tasks that don't need monitoring
    """
    try:
        # Create mission
        mission = orch.create_mission(objective=objective)

        # Create single-agent swarm
        swarm = orch.create_swarm(
            mission=mission,
            swarm_size="single",
            primary_mos=MOSEnum.INFANTRY,
        )

        # Activate and execute
        orch.activate_swarm(swarm.id)
        result = orch.execute_mission(swarm.id)

        # Disband
        aar = orch.disband_swarm(swarm.id)

        return {
            "objective": objective,
            "success": result["success"],
            "result": result.get("result"),
            "error": result.get("error"),
            "aar": aar.dict(),
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))
