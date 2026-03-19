-- Add org_id to all tables to enable org-wide stats aggregation.
-- Cross-service UUID referencing aura-network org, no FK constraint.

ALTER TABLE project_agents ADD COLUMN org_id UUID;
ALTER TABLE specs ADD COLUMN org_id UUID;
ALTER TABLE tasks ADD COLUMN org_id UUID;
ALTER TABLE sessions ADD COLUMN org_id UUID;
ALTER TABLE messages ADD COLUMN org_id UUID;
ALTER TABLE log_entries ADD COLUMN org_id UUID;

CREATE INDEX idx_project_agents_org_id ON project_agents (org_id);
CREATE INDEX idx_tasks_org_id ON tasks (org_id);
CREATE INDEX idx_sessions_org_id ON sessions (org_id);
CREATE INDEX idx_messages_org_id ON messages (org_id);
