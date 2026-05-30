-- Index for the user-scoped session list endpoint (chat-app left
-- panel). The chat app shows every session the current user has
-- created across every agent + project; this used to require
-- aura-os-server to fan out one /api/projects/:p/agents/:a/sessions
-- call per (agent, project_binding) pair (see
-- apps/chat-app/components/ChatAppLeftPanel/ChatAppLeftPanel.tsx
-- and stores/sessions-list-store.ts::loadAgentSessions in aura-os).
-- The new /api/me/sessions endpoint runs a single indexed query
-- against this partial index instead. WHERE event_count > 0 keeps
-- orphan empty rows out of the index entirely so the chat-app
-- left-panel default path (include_empty=false) never has to
-- visit them. The (created_by, last_event_at DESC NULLS LAST,
-- started_at DESC) shape mirrors idx_sessions_pa_recent and
-- idx_sessions_project_recent from migration 0015 so the same
-- ORDER BY plan is reused.
CREATE INDEX idx_sessions_user_recent
    ON sessions (created_by, last_event_at DESC NULLS LAST, started_at DESC)
    WHERE event_count > 0;
