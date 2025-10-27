import torch
import torch.nn as nn
from utils.base_agent import BaseAgent
from typing import List, Any
import pandas as pd
import numpy as np

class PredictiveModel(nn.Module):
    def __init__(self, input_size, hidden_size, output_size):
        super(PredictiveModel, self).__init__()
        self.lstm = nn.LSTM(input_size, hidden_size, batch_first=True)
        self.fc = nn.Linear(hidden_size, output_size)

    def forward(self, x):
        out, _ = self.lstm(x)
        out = self.fc(out[:, -1, :])
        return out

class PredictiveAnalystAgent(BaseAgent):
    def __init__(self, llm_clients):
        super().__init__("Predictive Analyst", llm_clients)
        self.model = PredictiveModel(input_size=1, hidden_size=50, output_size=1).to(self.device)

    def process(self, input_data: Any) -> Any:
        if isinstance(input_data, pd.DataFrame):
            # Assume time series data
            predictions = self.forecast(input_data)
            analysis = self.analyze_predictions(predictions)
            return {"predictions": predictions, "analysis": analysis}
        else:
            return "Invalid input for predictive analysis"

    def forecast(self, data: pd.DataFrame) -> List[float]:
        # Simple forecasting using LSTM
        # This is a placeholder, in practice train the model
        values = data.iloc[:, 0].values.astype(float)
        # Normalize
        values = (values - np.mean(values)) / np.std(values)
        # Convert to tensor
        x = torch.tensor(values[:-1], dtype=torch.float32).unsqueeze(0).unsqueeze(-1).to(self.device)
        with torch.no_grad():
            pred = self.model(x).cpu().numpy().flatten()
        return pred.tolist()

    def analyze_predictions(self, predictions: List[float]) -> str:
        prompt = f"Analyze these predictions: {predictions}. What trends do you see?"
        responses = [self.query_llm(llm, prompt) for llm in ["openai", "anthropic", "google"]]
        return self.combine_llm_responses(responses)