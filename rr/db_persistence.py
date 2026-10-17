"""
Database persistence layer for Rational Reserve
Author: Jason M Jarmacz | Evolution Strategist
Co-Author: Claude.ai | by Anthropic
"""

import os
from typing import Dict, List, Optional, Any
from datetime import datetime
import psycopg2
from psycopg2.extras import RealDictCursor, Json
import uuid
from loguru import logger

from .models import (
    Mission, Task, RankEnum, MOSEnum, StatusEnum, PriorityEnum
)


class RRDatabase:
    """PostgreSQL persistence for Rational Reserve system"""

    def __init__(self, connection_string: str = None):
        """Initialize database connection"""
        self.conn_string = connection_string or os.getenv(
            "RR_DATABASE_URL",
            "postgresql://postgres:postgres@localhost:5432/rr_swarm"
        )
        self.conn = None
        self._connect()

    def _connect(self):
        """Establish database connection"""
        try:
            self.conn = psycopg2.connect(self.conn_string)
            logger.info("RRDatabase connected to PostgreSQL")
        except Exception as e:
            logger.error(f"Database connection failed: {e}")
            raise

    def _execute(self, query: str, params: tuple = None, fetch: bool = False):
        """Execute query with error handling"""
        try:
            with self.conn.cursor(cursor_factory=RealDictCursor) as cur:
                cur.execute(query, params)
                if fetch:
                    return cur.fetchall()
                self.conn.commit()
                return True
        except Exception as e:
            self.conn.rollback()
            logger.error(f"Query execution failed: {e}")
            raise

    # ========== AGENT OPERATIONS ==========

    def save_agent(self, agent_id: str, name: str, rank: str, mos: str,
                   status: str, superior_id: str = None, metadata: Dict = None):
        """Persist agent to database"""
        query = """
            INSERT INTO agents (id, name, rank, mos, status, superior_id, metadata)
            VALUES (%s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                rank = EXCLUDED.rank,
                mos = EXCLUDED.mos,
                status = EXCLUDED.status,
                superior_id = EXCLUDED.superior_id,
                metadata = EXCLUDED.metadata,
                updated_at = CURRENT_TIMESTAMP
        """
        self._execute(query, (agent_id, name, rank, mos, status, superior_id, Json(metadata or {})))
        logger.debug(f"Agent {name} [{rank}] saved to DB")

    def get_agent(self, agent_id: str) -> Optional[Dict]:
        """Retrieve agent by ID"""
        query = "SELECT * FROM agents WHERE id = %s"
        result = self._execute(query, (agent_id,), fetch=True)
        return dict(result[0]) if result else None

    def get_agents_by_rank(self, rank: str) -> List[Dict]:
        """Get all agents of specific rank"""
        query = "SELECT * FROM agents WHERE rank = %s"
        results = self._execute(query, (rank,), fetch=True)
        return [dict(r) for r in results]

    def get_agents_by_mos(self, mos: str) -> List[Dict]:
        """Get all agents with specific MOS"""
        query = "SELECT * FROM agents WHERE mos = %s"
        results = self._execute(query, (mos,), fetch=True)
        return [dict(r) for r in results]

    def update_agent_status(self, agent_id: str, status: str):
        """Update agent status"""
        query = "UPDATE agents SET status = %s, updated_at = CURRENT_TIMESTAMP WHERE id = %s"
        self._execute(query, (status, agent_id))

    def deactivate_agent(self, agent_id: str):
        """Mark agent as inactive"""
        self.update_agent_status(agent_id, "inactive")

    # ========== MISSION OPERATIONS ==========

    def save_mission(self, mission: Mission):
        """Persist mission to database"""
        query = """
            INSERT INTO missions (id, objective, description, priority, status,
                                 assigned_swarm, constraints, resources)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (id) DO UPDATE SET
                objective = EXCLUDED.objective,
                description = EXCLUDED.description,
                priority = EXCLUDED.priority,
                status = EXCLUDED.status,
                assigned_swarm = EXCLUDED.assigned_swarm,
                constraints = EXCLUDED.constraints,
                resources = EXCLUDED.resources,
                updated_at = CURRENT_TIMESTAMP
        """
        self._execute(query, (
            mission.id, mission.objective, mission.description,
            mission.priority.value, mission.status.value,
            mission.assigned_swarm, Json(mission.constraints), Json(mission.resources)
        ))
        logger.debug(f"Mission {mission.id} saved to DB")

    def get_mission(self, mission_id: str) -> Optional[Dict]:
        """Retrieve mission by ID"""
        query = "SELECT * FROM missions WHERE id = %s"
        result = self._execute(query, (mission_id,), fetch=True)
        return dict(result[0]) if result else None

    def update_mission_status(self, mission_id: str, status: str):
        """Update mission status"""
        query = "UPDATE missions SET status = %s, updated_at = CURRENT_TIMESTAMP WHERE id = %s"
        self._execute(query, (status, mission_id))

    def complete_mission(self, mission_id: str):
        """Mark mission as complete"""
        query = """
            UPDATE missions SET status = %s, completed_at = CURRENT_TIMESTAMP,
                               updated_at = CURRENT_TIMESTAMP
            WHERE id = %s
        """
        # Persist the legacy literal "complete" so existing queries keep working
        # (StatusEnum.COMPLETE is now an alias of REPORTING in memory).
        self._execute(query, ("complete", mission_id))

    # ========== TASK OPERATIONS ==========

    def save_task(self, task: Task, mission_id: str):
        """Persist task to database"""
        query = """
            INSERT INTO tasks (id, mission_id, description, assigned_to, status,
                              dependencies, result, error, metadata)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (id) DO UPDATE SET
                description = EXCLUDED.description,
                assigned_to = EXCLUDED.assigned_to,
                status = EXCLUDED.status,
                dependencies = EXCLUDED.dependencies,
                result = EXCLUDED.result,
                error = EXCLUDED.error,
                metadata = EXCLUDED.metadata
        """
        self._execute(query, (
            task.id, mission_id, task.description, task.assigned_to,
            task.status.value, Json(task.dependencies), Json(task.result),
            task.error, Json(task.metadata)
        ))

    def get_tasks_by_mission(self, mission_id: str) -> List[Dict]:
        """Get all tasks for a mission"""
        query = "SELECT * FROM tasks WHERE mission_id = %s ORDER BY created_at"
        results = self._execute(query, (mission_id,), fetch=True)
        return [dict(r) for r in results]

    def update_task_status(self, task_id: str, status: str, result: Any = None, error: str = None):
        """Update task status and result"""
        query = """
            UPDATE tasks SET status = %s, result = %s, error = %s,
                            completed_at = CASE WHEN %s IN ('complete', 'reporting', 'failed')
                                              THEN CURRENT_TIMESTAMP ELSE completed_at END
            WHERE id = %s
        """
        self._execute(query, (status, Json(result), error, status, task_id))

    # ========== SWARM OPERATIONS ==========

    def save_swarm(self, swarm_id: str, name: str, mission_id: str,
                   general_id: str, status: str, size: str, metadata: Dict = None):
        """Persist swarm to database"""
        query = """
            INSERT INTO swarms (id, name, mission_id, general_id, status, size, metadata)
            VALUES (%s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                mission_id = EXCLUDED.mission_id,
                general_id = EXCLUDED.general_id,
                status = EXCLUDED.status,
                size = EXCLUDED.size,
                metadata = EXCLUDED.metadata
        """
        self._execute(query, (swarm_id, name, mission_id, general_id, status, size, Json(metadata or {})))
        logger.debug(f"Swarm {name} saved to DB")

    def activate_swarm(self, swarm_id: str):
        """Mark swarm as activated"""
        query = "UPDATE swarms SET status = 'active', activated_at = CURRENT_TIMESTAMP WHERE id = %s"
        self._execute(query, (swarm_id,))

    def disband_swarm(self, swarm_id: str):
        """Mark swarm as disbanded"""
        query = "UPDATE swarms SET status = 'disbanded', disbanded_at = CURRENT_TIMESTAMP WHERE id = %s"
        self._execute(query, (swarm_id,))

    def get_swarm(self, swarm_id: str) -> Optional[Dict]:
        """Retrieve swarm by ID"""
        query = "SELECT * FROM swarms WHERE id = %s"
        result = self._execute(query, (swarm_id,), fetch=True)
        return dict(result[0]) if result else None

    def get_active_swarms(self) -> List[Dict]:
        """Get all active swarms"""
        query = "SELECT * FROM swarms WHERE status = 'active'"
        results = self._execute(query, fetch=True)
        return [dict(r) for r in results]

    # ========== COMMUNICATION LOGS ==========

    def log_communication(self, comm_type: str, from_agent: str, to_agent: str,
                         content: str, metadata: Dict = None):
        """Log communication (order, sitrep, casrep)"""
        query = """
            INSERT INTO communications (id, type, from_agent, to_agent, content, metadata)
            VALUES (%s, %s, %s, %s, %s, %s)
        """
        comm_id = str(uuid.uuid4())
        self._execute(query, (comm_id, comm_type, from_agent, to_agent, content, Json(metadata or {})))
        return comm_id

    def acknowledge_communication(self, comm_id: str):
        """Mark communication as acknowledged"""
        query = "UPDATE communications SET acknowledged = TRUE WHERE id = %s"
        self._execute(query, (comm_id,))

    def get_communications(self, agent_id: str = None, comm_type: str = None) -> List[Dict]:
        """Get communications, optionally filtered"""
        if agent_id and comm_type:
            query = "SELECT * FROM communications WHERE to_agent = %s AND type = %s ORDER BY created_at DESC"
            results = self._execute(query, (agent_id, comm_type), fetch=True)
        elif agent_id:
            query = "SELECT * FROM communications WHERE to_agent = %s OR from_agent = %s ORDER BY created_at DESC"
            results = self._execute(query, (agent_id, agent_id), fetch=True)
        elif comm_type:
            query = "SELECT * FROM communications WHERE type = %s ORDER BY created_at DESC"
            results = self._execute(query, (comm_type,), fetch=True)
        else:
            query = "SELECT * FROM communications ORDER BY created_at DESC LIMIT 100"
            results = self._execute(query, fetch=True)
        return [dict(r) for r in results]

    # ========== AAR OPERATIONS ==========

    def save_aar(self, mission_id: str, swarm_id: str, success: bool,
                 objectives_met: List, lessons_learned: List, recommendations: List,
                 statistics: Dict):
        """Save After Action Review"""
        query = """
            INSERT INTO aars (id, mission_id, swarm_id, success, objectives_met,
                             lessons_learned, recommendations, statistics)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        """
        aar_id = str(uuid.uuid4())
        self._execute(query, (
            aar_id, mission_id, swarm_id, success,
            Json(objectives_met), Json(lessons_learned),
            Json(recommendations), Json(statistics)
        ))
        logger.debug(f"AAR saved for mission {mission_id}")
        return aar_id

    def get_aar(self, mission_id: str) -> Optional[Dict]:
        """Retrieve AAR for mission"""
        query = "SELECT * FROM aars WHERE mission_id = %s"
        result = self._execute(query, (mission_id,), fetch=True)
        return dict(result[0]) if result else None

    # ========== METRICS OPERATIONS ==========

    def record_agent_metrics(self, agent_id: str, mission_id: str,
                            tasks_completed: int, tasks_failed: int,
                            avg_completion_time: Any, performance_score: float):
        """Record agent performance metrics"""
        query = """
            INSERT INTO agent_metrics (id, agent_id, mission_id, tasks_completed,
                                      tasks_failed, average_completion_time, performance_score)
            VALUES (%s, %s, %s, %s, %s, %s, %s)
        """
        metric_id = str(uuid.uuid4())
        self._execute(query, (
            metric_id, agent_id, mission_id, tasks_completed,
            tasks_failed, avg_completion_time, performance_score
        ))

    def get_agent_metrics(self, agent_id: str) -> List[Dict]:
        """Get all metrics for an agent"""
        query = "SELECT * FROM agent_metrics WHERE agent_id = %s ORDER BY recorded_at DESC"
        results = self._execute(query, (agent_id,), fetch=True)
        return [dict(r) for r in results]

    def close(self):
        """Close database connection"""
        if self.conn:
            self.conn.close()
            logger.info("RRDatabase connection closed")
