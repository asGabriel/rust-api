-- Queue-list redesign (see "Fila e rotação de quadra" in the matchmaking
-- skill). One migration for the whole schema change — the DB is reset for
-- this (app not yet in real use), so no data backfill.

-- The waiting queue is a list of individual players per session, not a
-- collection of pre-formed teams. One row per session player who is NOT
-- currently on a court and NOT holding one. `games_played` is maintained by
-- the application on every match a player finishes (not derived). Ordering
-- of "who enters next": pinned DESC, pinned_at ASC, games_played ASC,
-- enqueued_at ASC.
CREATE TABLE matchmaking.session_queue (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES matchmaking.session (id),
    player_id UUID NOT NULL,
    games_played SMALLINT NOT NULL DEFAULT 0,
    enqueued_at TIMESTAMPTZ NOT NULL,
    pinned BOOLEAN NOT NULL DEFAULT false,
    pinned_at TIMESTAMPTZ NULL,
    UNIQUE (session_id, player_id)
);

CREATE INDEX idx_session_queue_session_id ON matchmaking.session_queue (session_id);

-- A team row now exists only from the moment it's drafted for a court:
-- `WAITING` is gone, `DRAFT` takes its place. Priority is per-player
-- (session_queue.pinned), so team.priority is dropped. `court` records the
-- court a `Draft` is the pending challenger for, so `resolve_match_result`
-- doesn't draft a second challenger for a court that already has one
-- awaiting the operator's confirmation (NULL for a manual draft, and for
-- any team already Playing/Holding — the court is on the Match then).
ALTER TABLE matchmaking.team DROP CONSTRAINT IF EXISTS team_status_check;
ALTER TABLE matchmaking.team
    ADD CONSTRAINT team_status_check
    CHECK (status IN ('DRAFT', 'HOLDING', 'PLAYING', 'DISBANDED'));

ALTER TABLE matchmaking.team DROP COLUMN IF EXISTS priority;
ALTER TABLE matchmaking.team ADD COLUMN court SMALLINT NULL;
