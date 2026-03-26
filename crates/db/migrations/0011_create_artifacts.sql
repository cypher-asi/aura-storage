-- Artifacts table for storing generated images and 3D models.
-- Supports parent/child relationships for iteration tracking.
CREATE TABLE artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    org_id UUID,
    created_by UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('image', 'model')),
    name TEXT,
    description TEXT,
    asset_url TEXT NOT NULL,
    thumbnail_url TEXT,
    original_url TEXT,
    parent_id UUID REFERENCES artifacts(id),
    is_iteration BOOLEAN NOT NULL DEFAULT false,
    prompt TEXT,
    prompt_mode TEXT CHECK (prompt_mode IN ('new', 'remix', 'edit')),
    model TEXT,
    provider TEXT,
    meta JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_artifacts_project_id ON artifacts (project_id);
CREATE INDEX idx_artifacts_created_by ON artifacts (created_by);
CREATE INDEX idx_artifacts_parent_id ON artifacts (parent_id);
CREATE INDEX idx_artifacts_type ON artifacts (type);
