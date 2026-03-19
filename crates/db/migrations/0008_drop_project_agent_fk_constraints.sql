-- Drop FK constraints from sessions, messages, log_entries, and tasks
-- that reference project_agents(id). This allows project agents to be
-- freely deleted without cascading or blocking on related records.
-- The columns remain as bare UUIDs (same cross-service ref pattern).

ALTER TABLE sessions DROP CONSTRAINT IF EXISTS sessions_project_agent_id_fkey;
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_project_agent_id_fkey;
ALTER TABLE log_entries DROP CONSTRAINT IF EXISTS log_entries_project_agent_id_fkey;
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_assigned_project_agent_id_fkey;
