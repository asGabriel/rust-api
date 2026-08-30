-- The waiting queue is a list of individual players per session, not a
-- collection of pre-formed teams. Teams are only formed when entering a
-- court (see "Fila e rotação de quadra" in the matchmaking skill).
--
-- One row per session player who is NOT currently on a court and NOT
-- holding one. `games_played` is maintained by the application on every
-- match a player finishes (not derived). Ordering of "who enters next":
--   pinned DESC, pinned_at ASC, games_played ASC, enqueued_at ASC
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
