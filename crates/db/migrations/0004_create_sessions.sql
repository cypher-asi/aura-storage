CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_agent_id UUID NOT NULL REFERENCES project_agents(id),
    project_id UUID NOT NULL,
    created_by UUID NOT NULL,
    model TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'rolled_over')),
    total_input_tokens BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    context_usage REAL NOT NULL DEFAULT 0,
    summary TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ
);

CREATE INDEX idx_sessions_project_agent ON sessions (project_agent_id);
CREATE INDEX idx_sessions_project ON sessions (project_id);
