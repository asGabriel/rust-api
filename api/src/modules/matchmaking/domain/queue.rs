use chrono::{DateTime, Utc};
use serde::Serialize;
use util::getters;
use uuid::Uuid;

use crate::modules::matchmaking::domain::{
    partner_history::PartnerHistory,
    player::{Gender, Player},
    session::GameMode,
};

/// One waiting player in a session's queue (`matchmaking.session_queue`).
///
/// The queue is a flat list of individuals — teams are only formed when a
/// player is about to enter a court (see the matchmaking skill,
/// "Fila e rotação de quadra"). `games_played` is the number of matches this
/// player has finished in the session; it is maintained on every result,
/// never derived. It carries over when the player leaves the list for a
/// court and comes back.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
/// court right now, plus the session config (`game_mode`, `players_per_team`)
/// bound at construction. Wraps the raw rows so the "who enters next"
/// ordering and the challenger selection live behind one type instead of
/// loose functions. Built from a repository read.
pub struct SessionQueue {
    entries: Vec<QueueEntry>,
    game_mode: GameMode,
    players_per_team: u8,
}

impl SessionQueue {
    pub fn new(entries: Vec<QueueEntry>, game_mode: GameMode, players_per_team: u8) -> Self {
        Self {
            entries,
            game_mode,
            players_per_team,
        }
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

    /// The suggested next challenger: the players at the front of the queue
    /// that satisfy `game_mode`'s gender composition for one team of
    /// `players_per_team` — the first `players_per_team` in `Male`/`Female`/
    /// `Open`, or `players_per_team / 2` of each gender (each in its own
    /// order) in `Mixed`. Returns `None` when the queue doesn't hold enough
    /// players of a required gender: the court stays idle and the operator
    /// can still build the team manually (the manual path ignores
    /// `game_mode` — see the matchmaking skill).
    ///
    /// The roster comes back in queue order; this is deliberately dumb — it
    /// never reaches past the front of the list to find a fresher pairing.
    /// `repeats_partner` just flags (never blocks) a pairing already played
    /// this session, as a hint for the operator to swap.
    pub fn next_challenger(
        &self,
        players: &[Player],
        history: &PartnerHistory,
    ) -> Option<ChallengerSuggestion> {
        let per_team = self.players_per_team as usize;
        if per_team == 0 {
            return None;
        }

        let ordered = self.ordered();

        let player_ids: Vec<Uuid> = if self.game_mode.is_mixed() {
            let per_gender = per_team / 2;
            let take = |gender: Gender| -> Option<Vec<Uuid>> {
                let ids: Vec<Uuid> = ordered
                    .iter()
                    .filter(|entry| Self::gender_of(players, *entry.player_id()) == Some(gender))
                    .take(per_gender)
                    .map(|entry| *entry.player_id())
                    .collect();
                (ids.len() == per_gender).then_some(ids)
            };
            let mut ids = take(Gender::Male)?;
            ids.extend(take(Gender::Female)?);
            ids
        } else {
            let ids: Vec<Uuid> = ordered
                .iter()
                .take(per_team)
                .map(|entry| *entry.player_id())
                .collect();
            (ids.len() == per_team).then_some(ids)?
        };

        let repeats_partner = player_ids.iter().enumerate().any(|(i, a)| {
            player_ids[i + 1..]
                .iter()
                .any(|b| history.have_played_together(*a, *b))
        });

        Some(ChallengerSuggestion {
            player_ids,
            repeats_partner,
        })
    }

    fn gender_of(players: &[Player], player_id: Uuid) -> Option<Gender> {
        players
            .iter()
            .find(|player| *player.id() == player_id)
            .map(|player| *player.gender())
    }
}

/// The queue's suggestion for who enters a court next, for the operator to
/// confirm or edit. `repeats_partner` is advisory only.
#[derive(Debug, Clone)]
pub struct ChallengerSuggestion {
    pub player_ids: Vec<Uuid>,
    pub repeats_partner: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::matchmaking::domain::{matches::Match, team::Team};

    fn entry(games_played: u16, secs_ago: i64) -> QueueEntry {
        entry_for(Uuid::new_v4(), games_played, secs_ago)
    }

    fn entry_for(player_id: Uuid, games_played: u16, secs_ago: i64) -> QueueEntry {
        let mut e = QueueEntry::new(Uuid::new_v4(), player_id, games_played);
        e.enqueued_at = Utc::now() - chrono::Duration::seconds(secs_ago);
        e
    }

    fn ordered_ids(entries: Vec<QueueEntry>) -> Vec<Uuid> {
        SessionQueue::new(entries, GameMode::Open, 2)
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

    fn player(gender: Gender) -> Player {
        Player::new("Player".to_string(), gender)
    }

    /// `Male`/`Female`/`Open`: `next_challenger` takes the first
    /// `players_per_team` of the queue in order, ignoring gender.
    #[test]
    fn test_next_challenger_takes_front_of_queue_in_non_mixed_mode() {
        let a = player(Gender::Male);
        let b = player(Gender::Male);
        let c = player(Gender::Male);
        let players = vec![a.clone(), b.clone(), c.clone()];

        // c waited longest, then b, then a.
        let queue = SessionQueue::new(
            vec![
                entry_for(*a.id(), 0, 10),
                entry_for(*b.id(), 0, 30),
                entry_for(*c.id(), 0, 60),
            ],
            GameMode::Male,
            2,
        );

        let suggestion = queue
            .next_challenger(&players, &PartnerHistory::empty())
            .expect("three players in queue, two needed");
        assert_eq!(suggestion.player_ids, vec![*c.id(), *b.id()]);
        assert!(!suggestion.repeats_partner);
    }

    /// `Mixed`: one of each gender, each taken in its own queue order — a
    /// man ahead of the first woman in the list does not push her out.
    #[test]
    fn test_next_challenger_picks_one_of_each_gender_in_mixed_mode() {
        let m1 = player(Gender::Male);
        let m2 = player(Gender::Male);
        let f1 = player(Gender::Female);
        let players = vec![m1.clone(), m2.clone(), f1.clone()];

        let queue = SessionQueue::new(
            vec![
                entry_for(*m1.id(), 0, 90),
                entry_for(*m2.id(), 0, 60),
                entry_for(*f1.id(), 0, 30),
            ],
            GameMode::Mixed,
            2,
        );

        let suggestion = queue
            .next_challenger(&players, &PartnerHistory::empty())
            .expect("one man and one woman available");
        assert_eq!(suggestion.player_ids, vec![*m1.id(), *f1.id()]);
    }

    /// `Mixed` with nobody of a required gender in the queue → `None`
    /// (court stays idle; operator can still build it by hand).
    #[test]
    fn test_next_challenger_returns_none_when_a_gender_is_missing_in_mixed_mode() {
        let m1 = player(Gender::Male);
        let m2 = player(Gender::Male);
        let players = vec![m1.clone(), m2.clone()];

        let queue = SessionQueue::new(
            vec![entry_for(*m1.id(), 0, 30), entry_for(*m2.id(), 0, 10)],
            GameMode::Mixed,
            2,
        );

        assert!(queue
            .next_challenger(&players, &PartnerHistory::empty())
            .is_none());
    }

    /// Fewer players in the queue than `players_per_team` → `None`.
    #[test]
    fn test_next_challenger_returns_none_when_queue_too_short() {
        let a = player(Gender::Male);
        let players = vec![a.clone()];
        let queue = SessionQueue::new(vec![entry_for(*a.id(), 0, 10)], GameMode::Open, 2);

        assert!(queue
            .next_challenger(&players, &PartnerHistory::empty())
            .is_none());
    }

    /// `repeats_partner` is set (but a suggestion still returned) when the
    /// two front-of-queue players already played together this session.
    #[test]
    fn test_next_challenger_flags_a_repeated_pairing_without_blocking() {
        let a = player(Gender::Male);
        let b = player(Gender::Male);
        let players = vec![a.clone(), b.clone()];

        let played_team = Team::new(Uuid::new_v4(), vec![*a.id(), *b.id()]);
        let played_match =
            Match::new(Uuid::new_v4(), 1, *played_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[played_team], &[played_match]);

        let queue = SessionQueue::new(
            vec![entry_for(*a.id(), 1, 30), entry_for(*b.id(), 1, 10)],
            GameMode::Male,
            2,
        );

        let suggestion = queue
            .next_challenger(&players, &history)
            .expect("two players available — repeat is flagged, not blocked");
        assert_eq!(suggestion.player_ids, vec![*a.id(), *b.id()]);
        assert!(suggestion.repeats_partner);
    }

    /// A pinned player is picked first even with more games and less wait.
    #[test]
    fn test_next_challenger_respects_pinned_order() {
        let pinned = player(Gender::Male);
        let waiting_a = player(Gender::Male);
        let waiting_b = player(Gender::Male);
        let players = vec![pinned.clone(), waiting_a.clone(), waiting_b.clone()];

        let mut pinned_entry = entry_for(*pinned.id(), 9, 0);
        pinned_entry.set_pinned(true);

        let queue = SessionQueue::new(
            vec![
                entry_for(*waiting_a.id(), 0, 90),
                entry_for(*waiting_b.id(), 0, 60),
                pinned_entry,
            ],
            GameMode::Male,
            2,
        );

        let suggestion = queue
            .next_challenger(&players, &PartnerHistory::empty())
            .unwrap();
        assert_eq!(suggestion.player_ids[0], *pinned.id());
    }
}
