import asyncio
from typing import Callable, Any, Dict, List
from loguru import logger
from .base_agent import BaseAgent

class RecursionEngine:
    def __init__(self, max_depth: int = 10, timeout: int = 30):
        self.max_depth = max_depth
        self.timeout = timeout
        self.active_loops: Dict[str, asyncio.Task] = {}

    async def start_recursive_loop(self, loop_id: str, func: Callable, initial_data: Any, stop_condition: Callable[[Any], bool], *args, **kwargs):
        if loop_id in self.active_loops:
            logger.warning(f"Loop {loop_id} already running")
            return

        task = asyncio.create_task(self._run_loop(loop_id, func, initial_data, stop_condition, *args, **kwargs))
        self.active_loops[loop_id] = task
        logger.info(f"Started recursive loop {loop_id}")

    async def _run_loop(self, loop_id: str, func: Callable, data: Any, stop_condition: Callable[[Any], bool], depth: int = 0, *args, **kwargs):
        try:
            while not stop_condition(data) and depth < self.max_depth:
                logger.debug(f"Loop {loop_id} iteration {depth}")
                data = await func(data, *args, **kwargs)
                depth += 1
                await asyncio.sleep(0.1)  # Prevent tight loop
            logger.info(f"Loop {loop_id} completed at depth {depth}")
        except Exception as e:
            logger.error(f"Error in loop {loop_id}: {e}")
        finally:
            del self.active_loops[loop_id]

    async def close_loop(self, loop_id: str):
        if loop_id in self.active_loops:
            self.active_loops[loop_id].cancel()
            del self.active_loops[loop_id]
            logger.info(f"Closed loop {loop_id}")

    async def optimize_with_recursion(self, domain: str, data: Any, llm_clients: Dict[str, Any]) -> Any:
        # Example optimization for different domains
        if domain == "threat_intelligence":
            return await self._threat_intelligence_loop(data, llm_clients)
        elif domain == "content_generation":
            return await self._content_generation_loop(data, llm_clients)
        elif domain == "coding_errors":
            return await self._coding_error_loop(data, llm_clients)
        else:
            return data

    async def _threat_intelligence_loop(self, data: Dict, llm_clients: Dict[str, Any]) -> Dict:
        # Recursive threat analysis
        prompt = f"Analyze threats in: {data.get('input', '')}"
        responses = []
        for name, client in llm_clients.items():
            try:
                response = client.query(prompt)
                responses.append(response)
            except:
                pass
        data['analysis'] = " ".join(responses)
        return data

    async def _content_generation_loop(self, data: Dict, llm_clients: Dict[str, Any]) -> Dict:
        # Recursive content improvement
        content = data.get('content', '')
        prompt = f"Improve this content: {content}"
        improved = []
        for name, client in llm_clients.items():
            try:
                response = client.query(prompt)
                improved.append(response)
            except:
                pass
        data['improved_content'] = " ".join(improved)
        return data

    async def _coding_error_loop(self, data: Dict, llm_clients: Dict[str, Any]) -> Dict:
        # Recursive code debugging
        code = data.get('code', '')
        prompt = f"Debug and fix this code: {code}"
        fixes = []
        for name, client in llm_clients.items():
            try:
                response = client.query(prompt)
                fixes.append(response)
            except:
                pass
        data['fixed_code'] = " ".join(fixes)
        return data