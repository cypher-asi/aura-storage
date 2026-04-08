-- Process workflow system: definitions, nodes, connections, runs, events, artifacts.
-- Processes are org-scoped with optional project linking.

CREATE TABLE process_folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    created_by UUID NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_process_folders_org_id ON process_folders(org_id);

CREATE TABLE processes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    created_by UUID NOT NULL,
    project_id UUID,
    folder_id UUID REFERENCES process_folders(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    schedule TEXT,
    tags JSONB NOT NULL DEFAULT '[]',
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_processes_org_id ON processes(org_id);
CREATE INDEX idx_processes_project_id ON processes(project_id);

CREATE TABLE process_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id UUID NOT NULL REFERENCES processes(id) ON DELETE CASCADE,
    node_type TEXT NOT NULL CHECK (node_type IN ('ignition','action','condition','artifact','delay','merge','prompt','sub_process','for_each')),
    label TEXT NOT NULL DEFAULT '',
    agent_id UUID,
    prompt TEXT NOT NULL DEFAULT '',
    config JSONB NOT NULL DEFAULT '{}',
    position_x DOUBLE PRECISION NOT NULL DEFAULT 0,
    position_y DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_process_nodes_process_id ON process_nodes(process_id);

CREATE TABLE process_node_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id UUID NOT NULL REFERENCES processes(id) ON DELETE CASCADE,
    source_node_id UUID NOT NULL REFERENCES process_nodes(id) ON DELETE CASCADE,
    source_handle TEXT,
    target_node_id UUID NOT NULL REFERENCES process_nodes(id) ON DELETE CASCADE,
    target_handle TEXT
);
CREATE INDEX idx_process_connections_process_id ON process_node_connections(process_id);

CREATE TABLE process_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id UUID NOT NULL REFERENCES processes(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed','failed','cancelled')),
    trigger TEXT NOT NULL DEFAULT 'manual' CHECK (trigger IN ('scheduled','manual')),
    error TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    total_input_tokens BIGINT DEFAULT 0,
    total_output_tokens BIGINT DEFAULT 0,
    cost_usd DOUBLE PRECISION,
    output TEXT,
    parent_run_id UUID REFERENCES process_runs(id),
    input_override TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_process_runs_process_id ON process_runs(process_id);

CREATE TABLE process_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES process_runs(id) ON DELETE CASCADE,
    node_id UUID NOT NULL,
    process_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed','failed','skipped')),
    input_snapshot TEXT NOT NULL DEFAULT '',
    output TEXT NOT NULL DEFAULT '',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    input_tokens BIGINT,
    output_tokens BIGINT,
    model TEXT,
    content_blocks JSONB
);
CREATE INDEX idx_process_events_run_id ON process_events(run_id);
CREATE INDEX idx_process_events_process_id ON process_events(process_id);

CREATE TABLE process_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id UUID NOT NULL,
    run_id UUID NOT NULL REFERENCES process_runs(id) ON DELETE CASCADE,
    node_id UUID NOT NULL,
    artifact_type TEXT NOT NULL CHECK (artifact_type IN ('report','document','data','media','code','custom')),
    name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_process_artifacts_run_id ON process_artifacts(run_id);
CREATE INDEX idx_process_artifacts_process_id ON process_artifacts(process_id);
