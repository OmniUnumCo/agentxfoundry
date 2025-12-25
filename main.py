from dotenv import load_dotenv
load_dotenv()

from fastapi import FastAPI, Depends
from llm_integrations.clients import get_llm_clients
from agents.predictive.predictive_agent import PredictiveAnalystAgent
from utils.recursion_engine import RecursionEngine
from utils.self_preservation import SelfPreservation
from utils.security import verify_token

# Rational Reserve imports
from rr.agent_factory import AgentFactory
from rr.swarm_orchestrator import SwarmOrchestrator
from rr.c2_router import C2Router
from rr import api_routes as rr_routes

app = FastAPI(title="Agent Ministry - Rational Reserve", version="1.0.0")

# Initialize core components
llm_clients = get_llm_clients()
recursion_engine = RecursionEngine()
self_preservation = SelfPreservation()

# Initialize Rational Reserve system
agent_factory = AgentFactory(llm_clients)
c2_router = C2Router(agent_factory)
swarm_orchestrator = SwarmOrchestrator(agent_factory)

# Set orchestrator for RR routes
rr_routes.set_orchestrator(swarm_orchestrator)

# Include RR routes
app.include_router(rr_routes.router)

# Initialize legacy agents
predictive_agent = PredictiveAnalystAgent(llm_clients)

@app.get("/")
def read_root():
    return {"message": "Agent Ministry - Rational Reserve API"}

@app.post("/predict")
def predict(data: dict, token: str = Depends(verify_token)):
    result = predictive_agent.process(data)
    return result

# Add more endpoints for other agents

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)