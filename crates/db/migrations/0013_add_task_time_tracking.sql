-- Per-task wall-clock timing. Populated by tasks repo on status transitions.
-- started_at: NOW() when task transitions to 'in_progress' (first time only)
-- ended_at:   NOW() when task transitions to 'done' or 'failed' (first time only)
-- Both nullable so historical tasks remain valid; queries that need a duration
-- coalesce ended_at to NOW() for in-progress tasks.
ALTER TABLE tasks
    ADD COLUMN started_at TIMESTAMPTZ,
    ADD COLUMN ended_at TIMESTAMPTZ;
