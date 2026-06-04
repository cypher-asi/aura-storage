-- Notes entity family: notes, note folders, and note comments.
-- Note BODIES live on S3 (managed by another service); aura-storage only
-- stores metadata plus an S3 reference (body_url, body_s3_key).
-- Blog posts are notes with extra blog fields and a draft/published lifecycle.

-- Folders form a tree via the self-referencing parent_id.
CREATE TABLE notes_folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    org_id UUID,
    parent_id UUID REFERENCES notes_folders(id),
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    org_id UUID,
    folder_id UUID REFERENCES notes_folders(id),
    title TEXT NOT NULL,
    slug TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    word_count INTEGER NOT NULL DEFAULT 0,
    body_url TEXT,
    body_s3_key TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    blog_type TEXT,
    excerpt TEXT,
    hero_image_url TEXT,
    read_time_minutes INTEGER,
    published_at TIMESTAMPTZ,
    author_id UUID,
    author_name TEXT,
    author_avatar_url TEXT,
    sections JSONB,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE note_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    note_id UUID NOT NULL REFERENCES notes(id),
    author_id UUID,
    author_name TEXT,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notes_project_id ON notes (project_id);
CREATE INDEX idx_notes_folder_id ON notes (folder_id);
CREATE INDEX idx_notes_project_status ON notes (project_id, status);
CREATE INDEX idx_notes_folders_project_id ON notes_folders (project_id);
CREATE INDEX idx_notes_folders_parent_id ON notes_folders (parent_id);
CREATE INDEX idx_note_comments_note_id ON note_comments (note_id);
