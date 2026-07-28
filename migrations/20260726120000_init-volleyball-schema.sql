CREATE SCHEMA IF NOT EXISTS volleyball;

CREATE TABLE IF NOT EXISTS volleyball.player (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    deleted_at TIMESTAMPTZ NULL
);

CREATE TABLE IF NOT EXISTS volleyball.session (
    id UUID PRIMARY KEY,
    session_date DATE NOT NULL,
    status TEXT NOT NULL DEFAULT 'OPEN',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL
);

CREATE TABLE IF NOT EXISTS volleyball.session_player (
    session_id UUID NOT NULL REFERENCES volleyball.session(id),
    player_id UUID NOT NULL REFERENCES volleyball.player(id),
    PRIMARY KEY (session_id, player_id)
);

CREATE TABLE IF NOT EXISTS volleyball.game (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES volleyball.session(id),
    court SMALLINT NOT NULL CHECK (court IN (1, 2)),
    team_a_player1_id UUID NOT NULL REFERENCES volleyball.player(id),
    team_a_player2_id UUID NOT NULL REFERENCES volleyball.player(id),
    team_b_player1_id UUID NOT NULL REFERENCES volleyball.player(id),
    team_b_player2_id UUID NOT NULL REFERENCES volleyball.player(id),
    winner TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_session_player_player_id ON volleyball.session_player(player_id);
CREATE INDEX IF NOT EXISTS idx_game_session_id ON volleyball.game(session_id);
CREATE INDEX IF NOT EXISTS idx_game_session_court ON volleyball.game(session_id, court);
CREATE INDEX IF NOT EXISTS idx_game_session_finished_at ON volleyball.game(session_id, finished_at);
CREATE INDEX IF NOT EXISTS idx_game_session_pending ON volleyball.game(session_id) WHERE winner IS NULL;
