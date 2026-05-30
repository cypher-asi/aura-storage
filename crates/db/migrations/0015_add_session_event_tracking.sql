-- Track event activity directly on the sessions row so the chat-app
-- session list ("Chats" sidekick, project session list, chat-app left
-- panel) can be served by a single indexed SELECT instead of fanning
-- out one `list_events?limit=1` HTTP probe per session in
-- aura-os-server. The probe is N+1 in the number of sessions and was
-- the dominant contributor to a slow first paint on agents with
-- meaningful chat history.
--
-- `event_count` and `last_event_at` are maintained by an
-- `AFTER INSERT` trigger on `session_events` so application code can't
-- forget to bump them. Backfill is a one-shot at migration time using
-- the existing `session_events` rows.
--
-- Statement order matters: the trigger is created BEFORE the backfill
-- runs. CREATE TRIGGER takes SHARE ROW EXCLUSIVE on session_events,
-- which conflicts with the ROW EXCLUSIVE that INSERTs need and is
-- held for the rest of this transaction. That means no concurrent
-- INSERT into session_events can commit between the trigger going
-- live and this migration committing, so the backfill's per-row
-- COUNT(*) is taken from a frozen view of the table. INSERTs that
-- committed BEFORE the trigger statement are picked up by the
-- backfill snapshot; INSERTs that arrive AFTER are queued behind
-- the lock and fire the trigger correctly once this commits.

ALTER TABLE sessions
    ADD COLUMN event_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN last_event_at TIMESTAMPTZ;

-- Atomic, application-agnostic event-count maintenance. Runs inside
-- the same transaction as the INSERT so a failed UPDATE rolls back
-- the event row, which means the counters can never silently drift
-- from the actual event population. INSERT-only is intentional --
-- session_events is append-only by codebase invariant.
CREATE OR REPLACE FUNCTION bump_session_event_stats() RETURNS TRIGGER AS $$
BEGIN
    UPDATE sessions
       SET event_count   = event_count + 1,
           last_event_at = GREATEST(COALESCE(last_event_at, NEW.timestamp), NEW.timestamp)
     WHERE id = NEW.session_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER session_events_after_insert
    AFTER INSERT ON session_events
    FOR EACH ROW EXECUTE FUNCTION bump_session_event_stats();

UPDATE sessions s SET
    event_count = (SELECT COUNT(*)::INTEGER FROM session_events e WHERE e.session_id = s.id),
    last_event_at = (SELECT MAX(timestamp) FROM session_events e WHERE e.session_id = s.id);

-- Partial indexes covering the hot list paths: per-agent sessions
-- list ("Chats" tab), per-project sessions list (projects sidekick).
-- WHERE event_count > 0 keeps orphan empty-session rows out of the
-- index entirely so the query plan reads only navigable sessions.
CREATE INDEX idx_sessions_pa_recent
    ON sessions (project_agent_id, last_event_at DESC NULLS LAST, started_at DESC)
    WHERE event_count > 0;

CREATE INDEX idx_sessions_project_recent
    ON sessions (project_id, last_event_at DESC NULLS LAST, started_at DESC)
    WHERE event_count > 0;
