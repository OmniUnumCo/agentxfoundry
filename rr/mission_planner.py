"""
Autonomous Mission Planning Engine

Enables Generals to autonomously:
1. Analyze mission objectives
2. Decompose into executable tasks
3. Assign tasks to appropriate agents by MOS/rank
4. Plan task dependencies and sequencing
5. Monitor execution and adapt
"""

from typing import Dict, Any, List, Optional
from loguru import logger
import json

from .models import Mission, Task, RankEnum, MOSEnum, StatusEnum
from .general_mind import GeneralMind


class MissionPlanner:
    """
    Autonomous mission planning for General-ranked agents

    Converts high-level objectives into structured task plans
    """

    def __init__(self, llm_client: Any):
        """
        Initialize mission planner

        Args:
            llm_client: LLM client for strategic planning
        """
        self.llm_client = llm_client
        logger.info("MissionPlanner initialized")

    def plan_mission(
        self,
        mission: Mission,
        swarm_composition: Dict[str, List[str]]
    ) -> List[Task]:
        """
        Autonomously plan mission execution

        Args:
            mission: Mission to plan
            swarm_composition: Dict of {rank: [agent_ids], mos: [agent_ids]}

        Returns:
            List of planned tasks with assignments and dependencies
        """
        logger.info(f"Planning mission: {mission.objective}")

        # Build swarm composition context
        composition_summary = self._summarize_swarm_composition(swarm_composition)

        # Generate planning prompt using General Mind framework
        planning_prompt = GeneralMind.mission_decomposition_prompt(
            objective=mission.objective,
            constraints=mission.constraints,
            swarm_composition=composition_summary,
        )

        # Get strategic decomposition from LLM
        try:
            response = self.llm_client.query(planning_prompt)
            logger.info("Received mission decomposition from strategic planning")

            # Parse response into tasks
            tasks = self._parse_task_plan(response, mission.id)

            logger.info(f"Mission decomposed into {len(tasks)} tasks")
            return tasks

        except Exception as e:
            logger.error(f"Mission planning failed: {str(e)}")
            # Fallback: create single task for direct execution
            return [Task(
                description=mission.objective,
                assigned_to=None,
                metadata={"fallback": True, "error": str(e)}
            )]

    def adapt_plan(
        self,
        mission: Mission,
        current_tasks: List[Task],
        change_detected: str,
        swarm_composition: Dict[str, List[str]]
    ) -> List[Task]:
        """
        Adapt mission plan based on detected changes (REACTIVE COMPLEX)

        Args:
            mission: Mission being executed
            current_tasks: Current task list
            change_detected: Description of change requiring adaptation
            swarm_composition: Current swarm composition

        Returns:
            Updated task list with adaptations
        """
        logger.info(f"Adapting mission plan due to: {change_detected}")

        # Build current state
        current_state = {
            "mission": mission.objective,
            "tasks_completed": [t for t in current_tasks if t.status == StatusEnum.COMPLETE],
            "tasks_in_progress": [t for t in current_tasks if t.status == StatusEnum.ENGAGED],
            "tasks_pending": [t for t in current_tasks if t.status == StatusEnum.STANDBY],
            "tasks_failed": [t for t in current_tasks if t.status == StatusEnum.FAILED],
            "change": change_detected,
        }

        # Generate reactive adaptation prompt
        adapt_prompt = GeneralMind.reactive_complex_prompt(
            objective=mission.objective,
            current_state=current_state,
            change_detected=change_detected,
        )

        try:
            response = self.llm_client.query(adapt_prompt)
            logger.info("Received adaptation plan from reactive complex")

            # Parse updated task plan
            adapted_tasks = self._parse_adaptation(response, current_tasks, mission.id)

            logger.info(f"Plan adapted with {len(adapted_tasks)} updated tasks")
            return adapted_tasks

        except Exception as e:
            logger.error(f"Plan adaptation failed: {str(e)}")
            # Fallback: return current tasks unchanged
            return current_tasks

    def predict_execution(
        self,
        mission: Mission,
        current_tasks: List[Task],
        swarm_status: Dict[str, Any]
    ) -> Dict[str, Any]:
        """
        Predict future execution states (PREDICTIVE ENGINE)

        Args:
            mission: Mission being executed
            current_tasks: Current task list
            swarm_status: Current swarm status

        Returns:
            Prediction dict with future states and recommendations
        """
        logger.info("Generating execution prediction")

        # Build trajectory data
        trajectory = {
            "mission": mission.objective,
            "progress": self._calculate_progress(current_tasks),
            "velocity": self._calculate_velocity(current_tasks),
            "blockers": [t.description for t in current_tasks if t.status == StatusEnum.BLOCKED],
            "swarm_status": swarm_status,
        }

        # Generate predictive analysis prompt
        predict_prompt = GeneralMind.predictive_engine_prompt(
            objective=mission.objective,
            current_trajectory=trajectory,
            horizon_depth=3,
        )

        try:
            response = self.llm_client.query(predict_prompt)
            logger.info("Received execution prediction from predictive engine")

            # Parse prediction
            prediction = self._parse_prediction(response)

            return prediction

        except Exception as e:
            logger.error(f"Execution prediction failed: {str(e)}")
            return {
                "success": False,
                "error": str(e),
                "fallback": True,
            }

    def _summarize_swarm_composition(self, composition: Dict[str, List[str]]) -> Dict[str, Any]:
        """
        Summarize swarm composition for planning context

        Args:
            composition: Dict of {rank: [agent_ids], mos: [agent_ids]}

        Returns:
            Summary dict for prompt context
        """
        summary = {}

        # Count by rank
        if "by_rank" in composition:
            summary["ranks"] = {
                rank: len(agents)
                for rank, agents in composition["by_rank"].items()
            }

        # Count by MOS
        if "by_mos" in composition:
            summary["mos_specialties"] = {
                mos: len(agents)
                for mos, agents in composition["by_mos"].items()
            }

        # Total agents
        summary["total_agents"] = sum(summary.get("ranks", {}).values())

        return summary

    def _parse_task_plan(self, llm_response: str, mission_id: str) -> List[Task]:
        """
        Parse LLM response into structured task list

        Args:
            llm_response: Raw LLM response
            mission_id: Mission ID for task linking

        Returns:
            List of Task objects
        """
        tasks = []

        try:
            # Try to extract JSON from response
            json_start = llm_response.find("[")
            json_end = llm_response.rfind("]") + 1

            if json_start >= 0 and json_end > json_start:
                json_str = llm_response[json_start:json_end]
                task_dicts = json.loads(json_str)

                for task_dict in task_dicts:
                    task = Task(
                        description=task_dict.get("description", ""),
                        assigned_to=task_dict.get("assigned_agent_id"),
                        dependencies=task_dict.get("dependencies", []),
                        metadata={
                            "assigned_mos": task_dict.get("assigned_mos"),
                            "assigned_rank": task_dict.get("assigned_rank"),
                            "is_iterative": task_dict.get("is_iterative", False),
                            "pivot_triggers": task_dict.get("pivot_triggers", []),
                            "predicted_duration": task_dict.get("predicted_duration"),
                            "success_criteria": task_dict.get("success_criteria"),
                            "mission_id": mission_id,
                        }
                    )
                    tasks.append(task)

                logger.info(f"Parsed {len(tasks)} tasks from LLM response")

            else:
                logger.warning("No JSON found in LLM response, parsing as text")
                # Fallback: parse as text-based task list
                tasks = self._parse_text_tasks(llm_response, mission_id)

        except json.JSONDecodeError as e:
            logger.warning(f"JSON decode failed: {str(e)}, parsing as text")
            tasks = self._parse_text_tasks(llm_response, mission_id)

        except Exception as e:
            logger.error(f"Task parsing error: {str(e)}")
            # Ultimate fallback: single task
            tasks = [Task(
                description=llm_response[:500] if llm_response else "Execute mission objective",
                metadata={"parse_error": str(e), "mission_id": mission_id}
            )]

        return tasks

    def _parse_text_tasks(self, text: str, mission_id: str) -> List[Task]:
        """
        Parse text-based task description into Task objects

        Args:
            text: Text containing task descriptions
            mission_id: Mission ID

        Returns:
            List of Task objects
        """
        tasks = []
        lines = text.split("\n")

        task_markers = ["1.", "2.", "3.", "4.", "5.", "6.", "7.", "8.", "9.", "-", "*", "•"]

        for line in lines:
            line = line.strip()
            if any(line.startswith(marker) for marker in task_markers):
                # Remove marker
                for marker in task_markers:
                    if line.startswith(marker):
                        line = line[len(marker):].strip()
                        break

                if len(line) > 5:  # Minimum viable task description
                    tasks.append(Task(
                        description=line,
                        metadata={"mission_id": mission_id, "parsed_from_text": True}
                    ))

        if not tasks:
            # If no tasks parsed, treat entire text as single task
            tasks = [Task(
                description=text[:500],
                metadata={"mission_id": mission_id, "full_text_task": True}
            )]

        return tasks

    def _parse_adaptation(
        self,
        llm_response: str,
        current_tasks: List[Task],
        mission_id: str
    ) -> List[Task]:
        """
        Parse adaptation response and merge with current tasks

        Args:
            llm_response: LLM response with adaptation instructions
            current_tasks: Current task list
            mission_id: Mission ID

        Returns:
            Updated task list
        """
        # Extract new tasks from adaptation
        new_tasks = self._parse_task_plan(llm_response, mission_id)

        # Merge: keep completed tasks, add new tasks, update pending
        adapted_tasks = []

        # Keep all completed tasks
        adapted_tasks.extend([t for t in current_tasks if t.status == StatusEnum.COMPLETE])

        # Add new tasks from adaptation
        adapted_tasks.extend(new_tasks)

        return adapted_tasks

    def _parse_prediction(self, llm_response: str) -> Dict[str, Any]:
        """
        Parse prediction response

        Args:
            llm_response: LLM response with predictions

        Returns:
            Prediction dictionary
        """
        return {
            "success": True,
            "raw_prediction": llm_response,
            "t_plus_1": self._extract_state(llm_response, "T+1"),
            "t_plus_2": self._extract_state(llm_response, "T+2"),
            "t_plus_3": self._extract_state(llm_response, "T+3"),
            "recommended_action": self._extract_action(llm_response),
        }

    def _extract_state(self, text: str, state_marker: str) -> str:
        """Extract predicted state from text"""
        # Simple extraction - look for state marker and grab following text
        if state_marker in text:
            start = text.find(state_marker)
            # Get next 200 chars or until next state marker
            end = start + 200
            snippet = text[start:end]
            return snippet.strip()
        return "No prediction available"

    def _extract_action(self, text: str) -> str:
        """Extract recommended action from text"""
        action_markers = ["recommended action", "optimal action", "next move", "telegraph"]
        for marker in action_markers:
            if marker.lower() in text.lower():
                start = text.lower().find(marker.lower())
                snippet = text[start:start+300]
                return snippet.strip()
        return "Continue current course"

    def _calculate_progress(self, tasks: List[Task]) -> float:
        """Calculate overall progress from task list"""
        if not tasks:
            return 0.0

        completed = len([t for t in tasks if t.status == StatusEnum.COMPLETE])
        return completed / len(tasks)

    def _calculate_velocity(self, tasks: List[Task]) -> str:
        """Calculate execution velocity"""
        completed = len([t for t in tasks if t.status == StatusEnum.COMPLETE])
        in_progress = len([t for t in tasks if t.status == StatusEnum.ENGAGED])

        if completed > in_progress:
            return "accelerating"
        elif in_progress > completed:
            return "steady"
        else:
            return "decelerating"
