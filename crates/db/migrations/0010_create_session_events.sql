-- Session events: replaces messages table.
-- Linear stream of typed events per session.
CREATE TABLE session_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL,
    user_id UUID,
    agent_id UUID,
    sender TEXT CHECK (sender IN ('user', 'agent')),
    project_id UUID,
    org_id UUID,
    type TEXT NOT NULL,
    content JSONB,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_session_events_session ON session_events (session_id, timestamp);
CREATE INDEX idx_session_events_project ON session_events (project_id, timestamp);
CREATE INDEX idx_session_events_org ON session_events (org_id, timestamp);
