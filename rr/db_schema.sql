-- Rational Reserve Database Schema
-- PostgreSQL 15+

-- Agent Records Table
CREATE TABLE IF NOT EXISTS agents (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    rank VARCHAR(10) NOT NULL,
    mos VARCHAR(10) NOT NULL,
    status VARCHAR(50) NOT NULL,
    superior_id UUID REFERENCES agents(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_agents_rank ON agents(rank);
CREATE INDEX idx_agents_mos ON agents(mos);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_superior ON agents(superior_id);

-- Missions Table
CREATE TABLE IF NOT EXISTS missions (
    id UUID PRIMARY KEY,
    objective TEXT NOT NULL,
    description TEXT,
    priority VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    assigned_swarm UUID,
    constraints JSONB DEFAULT '{}'::jsonb,
    resources JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

CREATE INDEX idx_missions_status ON missions(status);
CREATE INDEX idx_missions_priority ON missions(priority);
CREATE INDEX idx_missions_swarm ON missions(assigned_swarm);

-- Tasks Table
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY,
    mission_id UUID REFERENCES missions(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    assigned_to UUID REFERENCES agents(id),
    status VARCHAR(50) NOT NULL,
    dependencies JSONB DEFAULT '[]'::jsonb,
    result JSONB,
    error TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

CREATE INDEX idx_tasks_mission ON tasks(mission_id);
CREATE INDEX idx_tasks_agent ON tasks(assigned_to);
CREATE INDEX idx_tasks_status ON tasks(status);

-- Swarms Table
CREATE TABLE IF NOT EXISTS swarms (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    mission_id UUID REFERENCES missions(id),
    general_id UUID REFERENCES agents(id),
    status VARCHAR(50) NOT NULL,
    size VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    activated_at TIMESTAMP,
    disbanded_at TIMESTAMP,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_swarms_mission ON swarms(mission_id);
CREATE INDEX idx_swarms_general ON swarms(general_id);
CREATE INDEX idx_swarms_status ON swarms(status);

-- Units Table (fire teams, squads, platoons)
CREATE TABLE IF NOT EXISTS units (
    id UUID PRIMARY KEY,
    swarm_id UUID REFERENCES swarms(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,
    leader_id UUID REFERENCES agents(id),
    members JSONB DEFAULT '[]'::jsonb,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_units_swarm ON units(swarm_id);
CREATE INDEX idx_units_leader ON units(leader_id);

-- Communication Logs (Orders, SITREPs, CASREPs)
CREATE TABLE IF NOT EXISTS communications (
    id UUID PRIMARY KEY,
    type VARCHAR(50) NOT NULL, -- 'order', 'sitrep', 'casrep'
    from_agent UUID REFERENCES agents(id),
    to_agent UUID REFERENCES agents(id),
    content TEXT NOT NULL,
    acknowledged BOOLEAN DEFAULT FALSE,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_comms_type ON communications(type);
CREATE INDEX idx_comms_from ON communications(from_agent);
CREATE INDEX idx_comms_to ON communications(to_agent);
CREATE INDEX idx_comms_created ON communications(created_at);

-- After Action Reviews
CREATE TABLE IF NOT EXISTS aars (
    id UUID PRIMARY KEY,
    mission_id UUID REFERENCES missions(id),
    swarm_id UUID REFERENCES swarms(id),
    success BOOLEAN NOT NULL,
    objectives_met JSONB DEFAULT '[]'::jsonb,
    lessons_learned JSONB DEFAULT '[]'::jsonb,
    recommendations JSONB DEFAULT '[]'::jsonb,
    statistics JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_aars_mission ON aars(mission_id);
CREATE INDEX idx_aars_swarm ON aars(swarm_id);

-- Agent Performance Metrics
CREATE TABLE IF NOT EXISTS agent_metrics (
    id UUID PRIMARY KEY,
    agent_id UUID REFERENCES agents(id) ON DELETE CASCADE,
    mission_id UUID REFERENCES missions(id),
    tasks_completed INTEGER DEFAULT 0,
    tasks_failed INTEGER DEFAULT 0,
    average_completion_time INTERVAL,
    performance_score NUMERIC(5,2),
    recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_metrics_agent ON agent_metrics(agent_id);
CREATE INDEX idx_metrics_mission ON agent_metrics(mission_id);
