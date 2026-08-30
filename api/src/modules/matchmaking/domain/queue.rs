use chrono::{DateTime, Utc};
use util::getters;
use uuid::Uuid;

/// One waiting player in a session's queue (`matchmaking.session_queue`).
///
/// The queue is a flat list of individuals — teams are only formed when a
/// player is about to enter a court (see the matchmaking skill,
/// "Fila e rotação de quadra"). `games_played` is the number of matches this
/// player has finished in the session; it is maintained on every result,
/// never derived. It carries over when the player leaves the list for a
/// court and comes back.
#[derive(Debug, Clone)]
pub struct QueueEntry {
    id: Uuid,
    session_id: Uuid,
    player_id: Uuid,
    games_played: u16,
    enqueued_at: DateTime<Utc>,
    pinned: bool,
    pinned_at: Option<DateTime<Utc>>,
}

impl QueueEntry {
    /// A player (re-)entering the queue now: `enqueued_at = now`, not pinned.
    /// `games_played` is 0 on the session's first fill and the player's
    /// prior count + 1 when they come back off a court.
    pub fn new(session_id: Uuid, player_id: Uuid, games_played: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            player_id,
            games_played,
            enqueued_at: Utc::now(),
            pinned: false,
            pinned_at: None,
        }
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        self.pinned = pinned;
        self.pinned_at = pinned.then(Utc::now);
    }

    /// Sort key for "who enters next", ascending = enters sooner:
    /// pinned players first (oldest pin first), then fewest games played,
    /// then longest waiting. Non-pinned entries always have `pinned_at =
    /// None`, which the leading `!pinned` flag already orders after every
    /// pinned entry, so the `Option` never has to compare `None` against a
    /// pinned `Some`. Only used by `SessionQueue::ordered` — the ordering
    /// rule lives in exactly one place.
    fn order_key(&self) -> (bool, Option<DateTime<Utc>>, u16, DateTime<Utc>) {
        (
            !self.pinned,
            self.pinned_at,
            self.games_played,
            self.enqueued_at,
        )
    }
}

getters! {
    QueueEntry {
        id: Uuid,
        session_id: Uuid,
        player_id: Uuid,
        games_played: u16,
        enqueued_at: DateTime<Utc>,
        pinned: bool,
        pinned_at: Option<DateTime<Utc>>,
    }
}

impl From<&sqlx::postgres::PgRow> for QueueEntry {
    fn from(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;

        Self {
            id: row.get("id"),
            session_id: row.get("session_id"),
            player_id: row.get("player_id"),
            games_played: row.get::<i16, _>("games_played") as u16,
            enqueued_at: row.get("enqueued_at"),
            pinned: row.get("pinned"),
            pinned_at: row.get("pinned_at"),
        }
    }
}

/// A session's waiting queue: the `QueueEntry` rows of players not on a
/// court right now, wrapped so the "who enters next" ordering — and, from
/// F2 on, the challenger selection — live behind one type instead of loose
/// functions. Built from a repository read; holds no session config yet
/// (`next_challenger` will bind `game_mode`/`players_per_team` here when it
/// lands).
pub struct SessionQueue {
    entries: Vec<QueueEntry>,
}

impl SessionQueue {
    pub fn new(entries: Vec<QueueEntry>) -> Self {
        Self { entries }
    }

    /// The entries in canonical "who enters next" order: pinned first
    /// (oldest pin first), then fewest games played, then longest waiting.
    pub fn ordered(&self) -> Vec<&QueueEntry> {
        let mut ordered: Vec<&QueueEntry> = self.entries.iter().collect();
        ordered.sort_by_key(|entry| entry.order_key());
        ordered
    }

    pub fn entries(&self) -> &[QueueEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(games_played: u16, secs_ago: i64) -> QueueEntry {
        let mut e = QueueEntry::new(Uuid::new_v4(), Uuid::new_v4(), games_played);
        e.enqueued_at = Utc::now() - chrono::Duration::seconds(secs_ago);
        e
    }

    fn ordered_ids(entries: Vec<QueueEntry>) -> Vec<Uuid> {
        SessionQueue::new(entries)
            .ordered()
            .iter()
            .map(|entry| *entry.id())
            .collect()
    }

    #[test]
    fn test_order_by_games_then_wait_time() {
        let a = entry(2, 100); // most games — last
        let b = entry(1, 10); // fewer games, but waited less than c
        let c = entry(1, 50); // fewer games, waited longest — first

        assert_eq!(
            ordered_ids(vec![a.clone(), b.clone(), c.clone()]),
            vec![*c.id(), *b.id(), *a.id()]
        );
    }

    #[test]
    fn test_pinned_jumps_ahead_regardless_of_games_or_wait() {
        let fresh_but_pinned = {
            let mut e = entry(9, 0);
            e.set_pinned(true);
            e
        };
        let long_waiting_no_games = entry(0, 999);

        assert_eq!(
            ordered_ids(vec![
                long_waiting_no_games.clone(),
                fresh_but_pinned.clone()
            ]),
            vec![*fresh_but_pinned.id(), *long_waiting_no_games.id()]
        );
    }

    #[test]
    fn test_pinned_ordered_by_oldest_pin_first() {
        let mut first = entry(0, 0);
        first.set_pinned(true);
        first.pinned_at = Some(Utc::now() - chrono::Duration::seconds(30));

        let mut second = entry(0, 0);
        second.set_pinned(true);
        second.pinned_at = Some(Utc::now() - chrono::Duration::seconds(5));

        assert_eq!(
            ordered_ids(vec![second.clone(), first.clone()]),
            vec![*first.id(), *second.id()]
        );
    }

    #[test]
    fn test_unpinning_clears_pinned_at() {
        let mut e = entry(0, 0);
        e.set_pinned(true);
        assert!(e.pinned_at().is_some());
        e.set_pinned(false);
        assert!(e.pinned_at().is_none());
        assert!(!e.pinned());
    }
}
