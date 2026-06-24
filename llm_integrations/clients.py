import os
import math
from typing import Dict, Any
from openai import OpenAI
from anthropic import Anthropic
import google.generativeai as genai
from groq import Groq
import requests
# Add other imports as needed

class LLMClient:
    def __init__(self, name: str, api_key: str = None):
        self.name = name
        self.timeout = self._load_timeout()
        self.api_key = api_key or self._load_api_key()
        self.client = self._init_client()

    @staticmethod
    def _extract_response(payload: Dict[str, Any]) -> str:
        """Ensure API payload contains a non-null 'response' field."""
        if "response" not in payload or payload.get("response") is None:
            raise ValueError("LLM response payload missing 'response' field")
        return str(payload.get("response"))

    @staticmethod
    def _load_timeout() -> float:
        """Load and validate HTTP timeout from env, defaulting safely on invalid values."""
        raw = os.getenv("LLM_HTTP_TIMEOUT", "30")
        try:
            timeout = float(raw)
        except (TypeError, ValueError):
            return 30.0

        if not math.isfinite(timeout):
            return 30.0

        if timeout < 1 or timeout > 300:
            return 30.0

        return timeout

    def _load_api_key(self) -> str:
        # Try file path from env
        key_file = os.getenv(f"{self.name.upper()}_API_KEY_FILE")
        if key_file and os.path.exists(key_file):
            with open(key_file, 'r') as f:
                return f.read().strip()
        # Try default secrets directory
        default_path = os.path.join(os.path.dirname(__file__), '..', 'secrets', f'{self.name}_api_key')
        if os.path.exists(default_path):
            with open(default_path, 'r') as f:
                return f.read().strip()
        # Try environment variable
        return os.getenv(f"{self.name.upper()}_API_KEY", "")

    def _init_client(self):
        if self.name == "openai":
            return OpenAI(api_key=self.api_key)
        elif self.name == "anthropic":
            return Anthropic(api_key=self.api_key)
        elif self.name == "google":
            genai.configure(api_key=self.api_key)
            return genai.GenerativeModel('gemini-pro')
        elif self.name == "groq":
            return Groq(api_key=self.api_key)
        elif self.name in ["ollama", "llama", "deepseek", "qwen"]:
            return "http://localhost:11434"  # Ollama endpoint
        # Add other LLMs
        else:
            raise NotImplementedError(f"Client for {self.name} not implemented")

    def query(self, prompt: str) -> str:
        if self.name == "openai":
            response = self.client.chat.completions.create(
                model="gpt-4",
                messages=[{"role": "user", "content": prompt}]
            )
            return response.choices[0].message.content
        elif self.name == "anthropic":
            response = self.client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=4096,
                messages=[{"role": "user", "content": prompt}]
            )
            return response.content[0].text
        elif self.name == "google":
            response = self.client.generate_content(prompt)
            return response.text
        elif self.name == "groq":
            response = self.client.chat.completions.create(
                model="mixtral-8x7b-32768",
                messages=[{"role": "user", "content": prompt}]
            )
            return response.choices[0].message.content
        elif self.name in ["ollama", "llama"]:
            resp = requests.post(
                f"{self.client}/api/generate",
                json={"model": "llama3:latest", "prompt": prompt, "stream": False},
                timeout=self.timeout,
            )
            resp.raise_for_status()
            return self._extract_response(resp.json())
        elif self.name == "deepseek":
            resp = requests.post(
                f"{self.client}/api/generate",
                json={"model": "deepseek-v3.1:671b-cloud", "prompt": prompt, "stream": False},
                timeout=self.timeout,
            )
            resp.raise_for_status()
            return self._extract_response(resp.json())
        elif self.name == "qwen":
            resp = requests.post(
                f"{self.client}/api/generate",
                json={"model": "qwen3-coder:480b-cloud", "prompt": prompt, "stream": False},
                timeout=self.timeout,
            )
            resp.raise_for_status()
            return self._extract_response(resp.json())
        # Add other implementations
        else:
            raise NotImplementedError(f"Query for {self.name} not implemented")

def get_llm_clients() -> Dict[str, LLMClient]:
    clients = {}
    llm_names = ["ollama", "llama", "deepseek", "qwen", "openai", "anthropic", "google", "groq"]
    for name in llm_names:
        try:
            clients[name] = LLMClient(name)
        except Exception as e:
            print(f"Failed to initialize {name}: {e}")
    return clients