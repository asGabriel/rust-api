ALTER TABLE matchmaking.session
    ADD COLUMN roster_player_ids UUID[] NOT NULL DEFAULT '{}';

-- Every player currently checked in (player_ids) is, by definition, on the
-- session roster. New rows start empty and the roster is filled via
-- PATCH /matchmaking/sessions/{id}.
UPDATE matchmaking.session SET roster_player_ids = player_ids;
