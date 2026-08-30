-- Teams are no longer a standing queue: waiting players live in
-- matchmaking.session_queue as individuals, and a team row exists only from
-- the moment it's drafted for a court. `WAITING` is gone, `DRAFT` takes its
-- place. Priority is now per-player (session_queue.pinned), so team.priority
-- is dropped.
--
-- The DB is reset for this change (app not yet in real use), so no data
-- backfill: existing WAITING rows, if any, are not migrated.
ALTER TABLE matchmaking.team DROP CONSTRAINT IF EXISTS team_status_check;
ALTER TABLE matchmaking.team
    ADD CONSTRAINT team_status_check
    CHECK (status IN ('DRAFT', 'HOLDING', 'PLAYING', 'DISBANDED'));

ALTER TABLE matchmaking.team DROP COLUMN IF EXISTS priority;
