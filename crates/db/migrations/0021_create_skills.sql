-- Canonical, authenticated skill definitions shared by Aura runtimes.
--
-- Harness installations remain runtime-local because their approved paths
-- and commands are device grants. Only portable definitions and assignments
-- are stored here.
CREATE TABLE skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID,
    created_by UUID NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    allowed_tools JSONB NOT NULL DEFAULT '[]'::jsonb,
    model TEXT,
    context TEXT,
    user_invocable BOOLEAN NOT NULL DEFAULT TRUE,
    model_invocable BOOLEAN NOT NULL DEFAULT FALSE,
    agent_target JSONB,
    revision BIGINT NOT NULL DEFAULT 1,
    content_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT skills_name_format CHECK (name ~ '^[a-z0-9]([a-z0-9-]{0,62}[a-z0-9])?$'),
    CONSTRAINT skills_allowed_tools_array CHECK (jsonb_typeof(allowed_tools) = 'array'),
    CONSTRAINT skills_revision_positive CHECK (revision > 0)
);

CREATE UNIQUE INDEX idx_skills_personal_name
    ON skills (created_by, name)
    WHERE org_id IS NULL AND deleted_at IS NULL;

CREATE UNIQUE INDEX idx_skills_org_name
    ON skills (org_id, name)
    WHERE org_id IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX idx_skills_personal_updated
    ON skills (created_by, updated_at DESC)
    WHERE org_id IS NULL AND deleted_at IS NULL;

CREATE INDEX idx_skills_org_updated
    ON skills (org_id, updated_at DESC)
    WHERE org_id IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE agent_skill_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL,
    org_id UUID,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (skill_id, agent_id)
);

CREATE INDEX idx_agent_skill_assignments_agent
    ON agent_skill_assignments (agent_id, created_at);

CREATE INDEX idx_agent_skill_assignments_creator
    ON agent_skill_assignments (created_by, created_at);
