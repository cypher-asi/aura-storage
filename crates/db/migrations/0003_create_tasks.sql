CREATE TABLE tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    spec_id UUID NOT NULL REFERENCES specs(id),
    created_by UUID NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'ready', 'in_progress', 'done', 'failed', 'blocked')),
    order_index INTEGER NOT NULL,
    dependency_task_ids JSONB NOT NULL DEFAULT '[]',
    parent_task_id UUID REFERENCES tasks(id),
    assigned_project_agent_id UUID REFERENCES project_agents(id),
    session_id UUID,
    execution_notes TEXT,
    files_changed JSONB,
    model TEXT,
    total_input_tokens BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_project ON tasks (project_id, order_index);
CREATE INDEX idx_tasks_spec ON tasks (spec_id);
CREATE INDEX idx_tasks_status ON tasks (project_id, status);
