ALTER TABLE project_agents DROP CONSTRAINT IF EXISTS project_agents_status_check;

ALTER TABLE project_agents
    ADD CONSTRAINT project_agents_status_check
    CHECK (status IN ('idle', 'working', 'blocked', 'stopped', 'error', 'archived'));
