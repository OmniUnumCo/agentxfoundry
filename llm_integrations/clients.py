import os
from typing import Dict, Any
from openai import OpenAI
from anthropic import Anthropic
import google.generativeai as genai
from groq import Groq
# Add other imports as needed

class LLMClient:
    def __init__(self, name: str, api_key: str = None):
        self.name = name
        self.api_key = api_key or self._load_api_key()
        self.client = self._init_client()

    def _load_api_key(self) -> str:
        key_file = os.getenv(f"{self.name.upper()}_API_KEY_FILE")
        if key_file and os.path.exists(key_file):
            with open(key_file, 'r') as f:
                return f.read().strip()
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
                model="claude-3-sonnet-20240229",
                max_tokens=1000,
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
        # Add other implementations
        else:
            raise NotImplementedError(f"Query for {self.name} not implemented")

def get_llm_clients() -> Dict[str, LLMClient]:
    clients = {}
    llm_names = ["openai", "anthropic", "google", "groq", "perplexity", "deepseek", "qwen", "llama", "mistral"]
    for name in llm_names:
        try:
            clients[name] = LLMClient(name)
        except Exception as e:
            print(f"Failed to initialize {name}: {e}")
    return clients