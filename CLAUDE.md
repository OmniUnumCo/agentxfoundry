# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AgentX Foundry is a multi-language AI agent platform with dual-track architecture:

1. **Python Backend**: FastAPI-based agent system with multi-LLM integration, featuring a predictive analytics agent with LSTM models, recursion engine for iterative processing, and security/self-preservation utilities
2. **TypeScript/Genkit Frontend**: Google Genkit-based character generation demo using Gemini

The project also includes a custom "J*" (Jasteri) language lexer and encryption system for code processing.

## Development Commands

### Python Backend

```bash
# Install dependencies
pip install -r requirements.txt

# Run the FastAPI server
python main.py
# or with uvicorn directly:
uvicorn main:app --host 0.0.0.0 --port 8000 --reload

# Docker setup
docker-compose up
```

### TypeScript/Genkit

```bash
# Install dependencies
npm install

# Build TypeScript
npm run build

# Run Genkit developer UI
npm run genkit:dev
```

### Jasteri.sh Utilities

```bash
# Generate ECC keys for admins
./Jasteri.sh generate-keys

# Lex Python code to J* format
./Jasteri.sh lex <file.py>

# Encrypt J* files
./Jasteri.sh encrypt <file.jstar> <admin_key.pem>
```

## Architecture

### Python Backend Structure

**Core Components:**
- `main.py`: FastAPI application entry point with agent initialization
- `llm_integrations/clients.py`: Unified LLM client wrapper supporting OpenAI, Anthropic, Google, Groq, and others
- `utils/base_agent.py`: Abstract base class for all agents with LLM query capabilities
- `utils/recursion_engine.py`: Async recursive processing engine for iterative optimization across domains (threat intelligence, content generation, code debugging)
- `utils/security.py`: JWT authentication, Fernet encryption, zero-trust verification
- `utils/self_preservation.py`: Threat detection, quarantine, self-healing with LLM-generated patches

**Agent System:**
- Agents inherit from `BaseAgent` and implement `process()` method
- Each agent has access to all initialized LLM clients via `self.llm_clients`
- Multi-LLM consensus pattern: query multiple LLMs and combine responses
- Example: `agents/predictive/predictive_agent.py` - LSTM-based time series forecasting with LLM analysis

**LLM Integration:**
- API keys loaded from environment variables or `secrets/` directory files
- Client initialization pattern: `LLMClient(name)` auto-loads keys from `{NAME}_API_KEY` env var or `{NAME}_API_KEY_FILE` path
- Supported LLMs: OpenAI (GPT-4), Anthropic (Claude), Google (Gemini), Groq (Mixtral), Perplexity, DeepSeek, Qwen, Llama, Mistral
- Unified `query(prompt)` interface across all clients

**Security Architecture:**
- Zero-trust model with continuous verification (placeholder in `security.py:77-80`)
- JWT-based authentication with 30-minute token expiration
- Fernet symmetric encryption for data at rest
- ECC (SECP256R1) for J* file signing in Jasteri system
- Secrets stored in `secrets/` directory (gitignored)

**Recursion Engine:**
- Async loop management with max depth limits and timeouts
- Domain-specific optimization: `optimize_with_recursion(domain, data, llm_clients)`
- Active loop tracking with safe cancellation

**Self-Preservation:**
- Threat signature matching in `detect_threat()`
- Automatic quarantine to `quarantine/` directory
- LLM-powered patch generation via `patch_and_heal()`
- System backup to `backups/` directory

### TypeScript/Genkit Structure

- `index.ts`: Genkit prompt definition for D&D character generation
- Uses Google AI plugin with Gemini 1.5 Flash
- Structured output with Zod schema validation
- Demo flow: inspiration input -> character with name, stats, description

### J* Language System

- Custom lexer wraps source code in J* codex format
- ECC-based signing (not encryption) of lexed files
- Admin keys: Jimmy Paras, Jason Jarmacz
- Output directory: `jstar_output/`

## Key Patterns

### Adding a New Agent

1. Create agent file in `agents/<domain>/<agent_name>.py`
2. Inherit from `BaseAgent` and implement `process(input_data)`
3. Use `self.query_llm(llm_name, prompt)` for LLM calls
4. Initialize in `main.py` and add API endpoint
5. Agent has access to `self.device` for PyTorch operations

### Multi-LLM Consensus Pattern

```python
responses = [self.query_llm(llm, prompt) for llm in ["openai", "anthropic", "google"]]
combined = self.combine_llm_responses(responses)
```

### Security-First Development

- All API endpoints should use `token: str = Depends(verify_token)` for authentication
- Use `encrypt_data()` / `decrypt_data()` for sensitive data
- Check `detect_threat()` before processing untrusted input
- Call `continuous_verification(user_id, action)` for critical operations

## Important Notes

- Project is in ALPHA development status (see SECURITY.md)
- Python environment uses Python 3.9+ (see venv structure)
- PyTorch operations auto-detect CUDA availability
- All agents use loguru for structured logging
- FastAPI runs on port 8000 by default
- Genkit dev UI provides testing playground for character generation
- API key management: prefer file-based keys in `secrets/` over environment variables
