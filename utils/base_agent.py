import torch
import torch.nn as nn
from abc import ABC, abstractmethod
from typing import Dict, Any, List
from loguru import logger

class BaseAgent(ABC):
    def __init__(self, name: str, llm_clients: Dict[str, Any]):
        self.name = name
        self.llm_clients = llm_clients
        self.device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
        logger.info(f"Initialized {self.name} agent on {self.device}")

    @abstractmethod
    def process(self, input_data: Any) -> Any:
        pass

    def query_llm(self, llm_name: str, prompt: str) -> str:
        if llm_name not in self.llm_clients:
            raise ValueError(f"LLM {llm_name} not available")
        client = self.llm_clients[llm_name]
        # Assuming each client has a query method
        return client.query(prompt)

    def combine_llm_responses(self, responses: List[str]) -> str:
        # Simple combination, can be enhanced
        return " ".join(responses)