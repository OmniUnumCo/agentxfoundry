from dotenv import load_dotenv
load_dotenv()

from fastapi import FastAPI, Depends
from llm_integrations.clients import get_llm_clients
from agents.predictive.predictive_agent import PredictiveAnalystAgent
from utils.recursion_engine import RecursionEngine
from utils.self_preservation import SelfPreservation
from utils.security import verify_token
# Import other agents as they are created

app = FastAPI(title="Agent Ministry - Rational Reserve", version="1.0.0")

# Initialize core components
llm_clients = get_llm_clients()
recursion_engine = RecursionEngine()
self_preservation = SelfPreservation()

# Initialize agents
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