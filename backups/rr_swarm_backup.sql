--
-- PostgreSQL database dump
--

\restrict fC6ng2YZc3KukLXAV0fslfbg9LZrcoHXdJfN4J5Iymqf0SVPZGYIuZ31DO7OIZj

-- Dumped from database version 15.15
-- Dumped by pg_dump version 15.15

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: aars; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.aars (
    id uuid NOT NULL,
    mission_id uuid,
    swarm_id uuid,
    success boolean NOT NULL,
    objectives_met jsonb DEFAULT '[]'::jsonb,
    lessons_learned jsonb DEFAULT '[]'::jsonb,
    recommendations jsonb DEFAULT '[]'::jsonb,
    statistics jsonb DEFAULT '{}'::jsonb,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.aars OWNER TO postgres;

--
-- Name: agent_metrics; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.agent_metrics (
    id uuid NOT NULL,
    agent_id uuid,
    mission_id uuid,
    tasks_completed integer DEFAULT 0,
    tasks_failed integer DEFAULT 0,
    average_completion_time interval,
    performance_score numeric(5,2),
    recorded_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.agent_metrics OWNER TO postgres;

--
-- Name: agents; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.agents (
    id uuid NOT NULL,
    name character varying(255) NOT NULL,
    rank character varying(10) NOT NULL,
    mos character varying(10) NOT NULL,
    status character varying(50) NOT NULL,
    superior_id uuid,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    metadata jsonb DEFAULT '{}'::jsonb
);


ALTER TABLE public.agents OWNER TO postgres;

--
-- Name: communications; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.communications (
    id uuid NOT NULL,
    type character varying(50) NOT NULL,
    from_agent uuid,
    to_agent uuid,
    content text NOT NULL,
    acknowledged boolean DEFAULT false,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.communications OWNER TO postgres;

--
-- Name: missions; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.missions (
    id uuid NOT NULL,
    objective text NOT NULL,
    description text,
    priority character varying(50) NOT NULL,
    status character varying(50) NOT NULL,
    assigned_swarm uuid,
    constraints jsonb DEFAULT '{}'::jsonb,
    resources jsonb DEFAULT '{}'::jsonb,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    completed_at timestamp without time zone
);


ALTER TABLE public.missions OWNER TO postgres;

--
-- Name: swarms; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.swarms (
    id uuid NOT NULL,
    name character varying(255) NOT NULL,
    mission_id uuid,
    general_id uuid,
    status character varying(50) NOT NULL,
    size character varying(50),
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    activated_at timestamp without time zone,
    disbanded_at timestamp without time zone,
    metadata jsonb DEFAULT '{}'::jsonb
);


ALTER TABLE public.swarms OWNER TO postgres;

--
-- Name: tasks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tasks (
    id uuid NOT NULL,
    mission_id uuid,
    description text NOT NULL,
    assigned_to uuid,
    status character varying(50) NOT NULL,
    dependencies jsonb DEFAULT '[]'::jsonb,
    result jsonb,
    error text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    completed_at timestamp without time zone
);


ALTER TABLE public.tasks OWNER TO postgres;

--
-- Name: units; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.units (
    id uuid NOT NULL,
    swarm_id uuid,
    name character varying(255) NOT NULL,
    type character varying(50) NOT NULL,
    leader_id uuid,
    members jsonb DEFAULT '[]'::jsonb,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.units OWNER TO postgres;

--
-- Data for Name: aars; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.aars (id, mission_id, swarm_id, success, objectives_met, lessons_learned, recommendations, statistics, created_at) FROM stdin;
\.


--
-- Data for Name: agent_metrics; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.agent_metrics (id, agent_id, mission_id, tasks_completed, tasks_failed, average_completion_time, performance_score, recorded_at) FROM stdin;
\.


--
-- Data for Name: agents; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.agents (id, name, rank, mos, status, superior_id, created_at, updated_at, metadata) FROM stdin;
\.


--
-- Data for Name: communications; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.communications (id, type, from_agent, to_agent, content, acknowledged, metadata, created_at) FROM stdin;
\.


--
-- Data for Name: missions; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.missions (id, objective, description, priority, status, assigned_swarm, constraints, resources, created_at, updated_at, completed_at) FROM stdin;
\.


--
-- Data for Name: swarms; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.swarms (id, name, mission_id, general_id, status, size, created_at, activated_at, disbanded_at, metadata) FROM stdin;
\.


--
-- Data for Name: tasks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tasks (id, mission_id, description, assigned_to, status, dependencies, result, error, metadata, created_at, completed_at) FROM stdin;
\.


--
-- Data for Name: units; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.units (id, swarm_id, name, type, leader_id, members, created_at) FROM stdin;
\.


--
-- Name: aars aars_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.aars
    ADD CONSTRAINT aars_pkey PRIMARY KEY (id);


--
-- Name: agent_metrics agent_metrics_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.agent_metrics
    ADD CONSTRAINT agent_metrics_pkey PRIMARY KEY (id);


--
-- Name: agents agents_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_pkey PRIMARY KEY (id);


--
-- Name: communications communications_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.communications
    ADD CONSTRAINT communications_pkey PRIMARY KEY (id);


--
-- Name: missions missions_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.missions
    ADD CONSTRAINT missions_pkey PRIMARY KEY (id);


--
-- Name: swarms swarms_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.swarms
    ADD CONSTRAINT swarms_pkey PRIMARY KEY (id);


--
-- Name: tasks tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);


--
-- Name: units units_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.units
    ADD CONSTRAINT units_pkey PRIMARY KEY (id);


--
-- Name: idx_aars_mission; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_aars_mission ON public.aars USING btree (mission_id);


--
-- Name: idx_aars_swarm; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_aars_swarm ON public.aars USING btree (swarm_id);


--
-- Name: idx_agents_mos; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_agents_mos ON public.agents USING btree (mos);


--
-- Name: idx_agents_rank; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_agents_rank ON public.agents USING btree (rank);


--
-- Name: idx_agents_status; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_agents_status ON public.agents USING btree (status);


--
-- Name: idx_agents_superior; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_agents_superior ON public.agents USING btree (superior_id);


--
-- Name: idx_comms_created; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_comms_created ON public.communications USING btree (created_at);


--
-- Name: idx_comms_from; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_comms_from ON public.communications USING btree (from_agent);


--
-- Name: idx_comms_to; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_comms_to ON public.communications USING btree (to_agent);


--
-- Name: idx_comms_type; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_comms_type ON public.communications USING btree (type);


--
-- Name: idx_metrics_agent; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_metrics_agent ON public.agent_metrics USING btree (agent_id);


--
-- Name: idx_metrics_mission; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_metrics_mission ON public.agent_metrics USING btree (mission_id);


--
-- Name: idx_missions_priority; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_missions_priority ON public.missions USING btree (priority);


--
-- Name: idx_missions_status; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_missions_status ON public.missions USING btree (status);


--
-- Name: idx_missions_swarm; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_missions_swarm ON public.missions USING btree (assigned_swarm);


--
-- Name: idx_swarms_general; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_swarms_general ON public.swarms USING btree (general_id);


--
-- Name: idx_swarms_mission; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_swarms_mission ON public.swarms USING btree (mission_id);


--
-- Name: idx_swarms_status; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_swarms_status ON public.swarms USING btree (status);


--
-- Name: idx_tasks_agent; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tasks_agent ON public.tasks USING btree (assigned_to);


--
-- Name: idx_tasks_mission; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tasks_mission ON public.tasks USING btree (mission_id);


--
-- Name: idx_tasks_status; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tasks_status ON public.tasks USING btree (status);


--
-- Name: idx_units_leader; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_units_leader ON public.units USING btree (leader_id);


--
-- Name: idx_units_swarm; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_units_swarm ON public.units USING btree (swarm_id);


--
-- Name: aars aars_mission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.aars
    ADD CONSTRAINT aars_mission_id_fkey FOREIGN KEY (mission_id) REFERENCES public.missions(id);


--
-- Name: aars aars_swarm_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.aars
    ADD CONSTRAINT aars_swarm_id_fkey FOREIGN KEY (swarm_id) REFERENCES public.swarms(id);


--
-- Name: agent_metrics agent_metrics_agent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.agent_metrics
    ADD CONSTRAINT agent_metrics_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES public.agents(id) ON DELETE CASCADE;


--
-- Name: agent_metrics agent_metrics_mission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.agent_metrics
    ADD CONSTRAINT agent_metrics_mission_id_fkey FOREIGN KEY (mission_id) REFERENCES public.missions(id);


--
-- Name: agents agents_superior_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.agents
    ADD CONSTRAINT agents_superior_id_fkey FOREIGN KEY (superior_id) REFERENCES public.agents(id);


--
-- Name: communications communications_from_agent_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.communications
    ADD CONSTRAINT communications_from_agent_fkey FOREIGN KEY (from_agent) REFERENCES public.agents(id);


--
-- Name: communications communications_to_agent_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.communications
    ADD CONSTRAINT communications_to_agent_fkey FOREIGN KEY (to_agent) REFERENCES public.agents(id);


--
-- Name: swarms swarms_general_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.swarms
    ADD CONSTRAINT swarms_general_id_fkey FOREIGN KEY (general_id) REFERENCES public.agents(id);


--
-- Name: swarms swarms_mission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.swarms
    ADD CONSTRAINT swarms_mission_id_fkey FOREIGN KEY (mission_id) REFERENCES public.missions(id);


--
-- Name: tasks tasks_assigned_to_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_assigned_to_fkey FOREIGN KEY (assigned_to) REFERENCES public.agents(id);


--
-- Name: tasks tasks_mission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_mission_id_fkey FOREIGN KEY (mission_id) REFERENCES public.missions(id) ON DELETE CASCADE;


--
-- Name: units units_leader_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.units
    ADD CONSTRAINT units_leader_id_fkey FOREIGN KEY (leader_id) REFERENCES public.agents(id);


--
-- Name: units units_swarm_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.units
    ADD CONSTRAINT units_swarm_id_fkey FOREIGN KEY (swarm_id) REFERENCES public.swarms(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict fC6ng2YZc3KukLXAV0fslfbg9LZrcoHXdJfN4J5Iymqf0SVPZGYIuZ31DO7OIZj

