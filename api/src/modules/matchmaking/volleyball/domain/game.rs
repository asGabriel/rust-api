use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pair {
    player1_id: Uuid,
    player2_id: Uuid,
}

impl Pair {
    pub fn new(player1_id: Uuid, player2_id: Uuid) -> Self {
        Self {
            player1_id,
            player2_id,
        }
    }

    pub fn players(&self) -> [Uuid; 2] {
        [self.player1_id, self.player2_id]
    }

    pub fn contains(&self, player_id: Uuid) -> bool {
        self.player1_id == player_id || self.player2_id == player_id
    }

    /// Order-independent equality: {A, B} == {B, A}.
    pub fn same_players_as(&self, other: &Pair) -> bool {
        self.contains(other.player1_id) && self.contains(other.player2_id)
    }
}

getters!(Pair {
    player1_id: Uuid,
    player2_id: Uuid,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GameWinner {
    TeamA,
    TeamB,
}

impl From<String> for GameWinner {
    fn from(s: String) -> Self {
        match s.as_str() {
            "TEAM_B" => GameWinner::TeamB,
            _ => GameWinner::TeamA,
        }
    }
}

impl From<GameWinner> for String {
    fn from(winner: GameWinner) -> Self {
        match winner {
            GameWinner::TeamA => "TEAM_A".to_string(),
            GameWinner::TeamB => "TEAM_B".to_string(),
        }
    }
}

impl std::fmt::Display for GameWinner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GameWinner::TeamA => "TEAM_A",
            GameWinner::TeamB => "TEAM_B",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    id: Uuid,
    session_id: Uuid,
    court: i16,
    team_a: Pair,
    team_b: Pair,
    winner: Option<GameWinner>,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

impl Game {
    pub fn new_pending(
        session_id: Uuid,
        court: i16,
        team_a: Pair,
        team_b: Pair,
    ) -> HttpResult<Self> {
        if !(1..=2).contains(&court) {
            return Err(Box::new(HttpError::bad_request(format!(
                "Invalid court: {}",
                court
            ))));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            session_id,
            court,
            team_a,
            team_b,
            winner: None,
            created_at: Utc::now(),
            finished_at: None,
        })
    }

    pub fn is_pending(&self) -> bool {
        self.winner.is_none()
    }

    pub fn record_result(&mut self, winner: GameWinner) -> HttpResult<()> {
        if !self.is_pending() {
            return Err(Box::new(HttpError::bad_request(
                "Game result already recorded",
            )));
        }

        self.winner = Some(winner);
        self.finished_at = Some(Utc::now());

        Ok(())
    }

    pub fn winning_pair(&self) -> Option<&Pair> {
        match self.winner {
            Some(GameWinner::TeamA) => Some(&self.team_a),
            Some(GameWinner::TeamB) => Some(&self.team_b),
            None => None,
        }
    }

    pub fn losing_pair(&self) -> Option<&Pair> {
        match self.winner {
            Some(GameWinner::TeamA) => Some(&self.team_b),
            Some(GameWinner::TeamB) => Some(&self.team_a),
            None => None,
        }
    }

    /// Pure business logic for rule #3: a pair that wins two matches in a row on
    /// the same court must leave too, same as the losing pair. `previous_finished_on_court`
    /// is the most recent *other* finished game on that same court (`None` if this
    /// was the court's first finished game).
    pub fn resolve_departures(
        &self,
        previous_finished_on_court: Option<&Game>,
    ) -> HttpResult<DepartureOutcome> {
        let winner_pair = self
            .winning_pair()
            .ok_or_else(|| Box::new(HttpError::bad_request("Game has no result yet")))?;
        let loser_pair = self
            .losing_pair()
            .expect("losing_pair is Some whenever winning_pair is Some");

        let is_second_consecutive_win = previous_finished_on_court
            .and_then(|previous| previous.winning_pair())
            .is_some_and(|previous_winner| previous_winner.same_players_as(winner_pair));

        if is_second_consecutive_win {
            let mut departing_player_ids = winner_pair.players().to_vec();
            departing_player_ids.extend(loser_pair.players());

            Ok(DepartureOutcome {
                court: self.court,
                departing_player_ids,
                retained_pair: None,
            })
        } else {
            Ok(DepartureOutcome {
                court: self.court,
                departing_player_ids: loser_pair.players().to_vec(),
                retained_pair: Some(*winner_pair),
            })
        }
    }
}

getters!(
    Game {
        id: Uuid,
        session_id: Uuid,
        court: i16,
        team_a: Pair,
        team_b: Pair,
        winner: Option<GameWinner>,
        created_at: DateTime<Utc>,
        finished_at: Option<DateTime<Utc>>,
    }
);

from_row_constructor! {
    Game {
        id: Uuid,
        session_id: Uuid,
        court: i16,
        team_a: Pair,
        team_b: Pair,
        winner: Option<GameWinner>,
        created_at: DateTime<Utc>,
        finished_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFilters {
    session_id: Option<Uuid>,
    court: Option<i16>,
    pending_only: Option<bool>,
}

impl GameFilters {
    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_optional_session_id(mut self, session_id: Option<Uuid>) -> Self {
        if let Some(session_id) = session_id {
            self.session_id = Some(session_id);
        }
        self
    }

    pub fn with_court(mut self, court: i16) -> Self {
        self.court = Some(court);
        self
    }

    pub fn with_optional_court(mut self, court: Option<i16>) -> Self {
        if let Some(court) = court {
            self.court = Some(court);
        }
        self
    }

    pub fn with_pending_only(mut self, pending_only: bool) -> Self {
        self.pending_only = Some(pending_only);
        self
    }

    pub fn with_optional_pending_only(mut self, pending_only: Option<bool>) -> Self {
        if let Some(pending_only) = pending_only {
            self.pending_only = Some(pending_only);
        }
        self
    }
}

getters!(
    GameFilters {
        session_id: Option<Uuid>,
        court: Option<i16>,
        pending_only: Option<bool>,
    }
);

/// Outcome of applying the "2 consecutive wins forces exit" rule to a
/// just-finished game. `departing_player_ids` has length 2 when only the
/// losing pair leaves, or length 4 when the winning pair is forced out too
/// (in which case `retained_pair` is `None` and the whole court needs a
/// fresh redraw).
#[derive(Debug, Clone)]
pub struct DepartureOutcome {
    pub court: i16,
    pub departing_player_ids: Vec<Uuid>,
    pub retained_pair: Option<Pair>,
}

impl DepartureOutcome {
    pub fn slots_needed(&self) -> usize {
        self.departing_player_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: u8, b: u8) -> Pair {
        Pair::new(uuid_from(a), uuid_from(b))
    }

    fn uuid_from(byte: u8) -> Uuid {
        Uuid::from_bytes([byte; 16])
    }

    fn finished_game(court: i16, team_a: Pair, team_b: Pair, winner: GameWinner) -> Game {
        let mut game = Game::new_pending(Uuid::new_v4(), court, team_a, team_b).unwrap();
        game.record_result(winner).unwrap();
        game
    }

    #[test]
    fn same_players_as_is_order_independent() {
        let p1 = pair(1, 2);
        let p2 = pair(2, 1);
        assert!(p1.same_players_as(&p2));
    }

    #[test]
    fn first_game_on_court_never_double_exits() {
        let game = finished_game(1, pair(1, 2), pair(3, 4), GameWinner::TeamA);

        let outcome = game.resolve_departures(None).unwrap();

        assert_eq!(outcome.slots_needed(), 2);
        assert_eq!(outcome.retained_pair, Some(pair(1, 2)));
    }

    #[test]
    fn same_pair_wins_twice_in_a_row_forces_double_exit() {
        let previous = finished_game(1, pair(5, 6), pair(1, 2), GameWinner::TeamB); // {1,2} just won
        let current = finished_game(1, pair(1, 2), pair(3, 4), GameWinner::TeamA); // {1,2} wins again -> 2nd win

        // previous winner is {1,2}, same as current winner -> forces double exit
        let outcome = current.resolve_departures(Some(&previous)).unwrap();
        assert_eq!(outcome.slots_needed(), 4);
        assert!(outcome.retained_pair.is_none());
    }

    #[test]
    fn different_winner_than_previous_is_first_win_not_double_exit() {
        let previous = finished_game(1, pair(5, 6), pair(7, 8), GameWinner::TeamA); // {5,6} won previously
        let current = finished_game(1, pair(1, 2), pair(3, 4), GameWinner::TeamA); // {1,2} wins now, different pair

        let outcome = current.resolve_departures(Some(&previous)).unwrap();
        assert_eq!(outcome.slots_needed(), 2);
        assert_eq!(outcome.retained_pair, Some(pair(1, 2)));
    }
}
