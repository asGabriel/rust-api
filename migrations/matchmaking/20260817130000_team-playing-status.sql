-- `PLAYING` is derived, never written by the application: a trigger on
-- `match` sets both teams' status to `PLAYING` the moment a match with no
-- winner yet is inserted, so "this team currently has an open match" can
-- never drift from `team.status` no matter what inserts the match row.
ALTER TABLE matchmaking.team DROP CONSTRAINT IF EXISTS team_status_check;
ALTER TABLE matchmaking.team
    ADD CONSTRAINT team_status_check
    CHECK (status IN ('WAITING', 'HOLDING', 'PLAYING', 'DISBANDED'));

CREATE OR REPLACE FUNCTION matchmaking.mark_teams_playing()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.winner_team_id IS NULL THEN
        UPDATE matchmaking.team
        SET status = 'PLAYING'
        WHERE id IN (NEW.team_a_id, NEW.team_b_id);
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_match_marks_teams_playing
    AFTER INSERT ON matchmaking.match
    FOR EACH ROW
    EXECUTE FUNCTION matchmaking.mark_teams_playing();
