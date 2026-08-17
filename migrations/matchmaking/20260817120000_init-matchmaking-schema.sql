CREATE SCHEMA IF NOT EXISTS matchmaking;

CREATE TABLE matchmaking.player (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    gender TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL
);

CREATE TABLE matchmaking.session (
    id UUID PRIMARY KEY,
    date DATE NOT NULL,
    players_per_team SMALLINT NOT NULL,
    sets_to_win SMALLINT NOT NULL,
    points_per_set SMALLINT NOT NULL,
    available_courts SMALLINT NOT NULL,
    game_mode TEXT NOT NULL,
    shuffle_type TEXT NOT NULL,
    player_ids UUID[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL
);

CREATE TABLE matchmaking.team (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES matchmaking.session (id),
    player_ids UUID[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL,
    consecutive_wins SMALLINT NOT NULL DEFAULT 0,
    priority BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_team_session_id ON matchmaking.team (session_id);

CREATE TABLE matchmaking.match (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES matchmaking.session (id),
    court SMALLINT NOT NULL,
    team_a_id UUID NOT NULL REFERENCES matchmaking.team (id),
    team_b_id UUID NOT NULL REFERENCES matchmaking.team (id),
    winner_team_id UUID NULL REFERENCES matchmaking.team (id),
    started_at TIMESTAMPTZ NOT NULL,
    played_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_match_session_id ON matchmaking.match (session_id);
