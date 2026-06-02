ALTER TABLE sessions
    ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN public_share_id TEXT;

CREATE UNIQUE INDEX idx_sessions_public_share_id
    ON sessions (public_share_id)
    WHERE public_share_id IS NOT NULL;
