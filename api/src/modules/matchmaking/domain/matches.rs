use std::collections::HashSet;

use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

use crate::modules::matchmaking::domain::team::Team;

/// A match assigned to a court: the two teams facing off, and the result
/// once it's been reported. `winner_team_id`/`played_at` are `None` while
/// the match is still in progress on the court.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    id: Uuid,
    session_id: Uuid,
    court: u8,
    team_a_id: Uuid,
    team_b_id: Uuid,
    winner_team_id: Option<Uuid>,
    started_at: DateTime<Utc>,
    played_at: Option<DateTime<Utc>>,
}

impl Match {
    pub fn new(session_id: Uuid, court: u8, team_a_id: Uuid, team_b_id: Uuid) -> HttpResult<Self> {
        if team_a_id == team_b_id {
            return Err(Box::new(HttpError::bad_request(
                "A team cannot play against itself",
            )));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            session_id,
            court,
            team_a_id,
            team_b_id,
            winner_team_id: None,
            started_at: Utc::now(),
            played_at: None,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.winner_team_id.is_some()
    }

    /// Whether `team_id` is one of the two teams playing this match.
    pub fn has_team(&self, team_id: Uuid) -> bool {
        team_id == self.team_a_id || team_id == self.team_b_id
    }

    /// Records the result of this in-progress match. Fails if a result was
    /// already reported, or if `winner_team_id` isn't one of the two teams
    /// that played it.
    pub fn finish(&mut self, winner_team_id: Uuid) -> HttpResult<()> {
        if self.is_finished() {
            return Err(Box::new(HttpError::conflict(
                "Match already has a recorded result",
            )));
        }

        if !self.has_team(winner_team_id) {
            return Err(Box::new(HttpError::bad_request(
                "winner_team_id must be one of the match's two teams",
            )));
        }

        self.winner_team_id = Some(winner_team_id);
        self.played_at = Some(Utc::now());

        Ok(())
    }

    /// The ids of every team currently tied up in an in-progress match,
    /// across any court — used to keep a team from being started on a
    /// second court before its current match is reported.
    pub fn busy_team_ids(matches: &[Match]) -> HashSet<Uuid> {
        matches
            .iter()
            .filter(|match_| !match_.is_finished())
            .flat_map(|match_| [*match_.team_a_id(), *match_.team_b_id()])
            .collect()
    }
}

/// Centralizes the checks for starting a match: both teams must belong to
/// this session, be a full team (not one still waiting on a partner), still
/// be around to play (not disbanded), and not already be busy in another
/// in-progress match. Bound to a `session_id` and `players_per_team` at
/// construction, same as `TeamValidator`.
pub struct MatchStartValidator {
    session_id: Uuid,
    players_per_team: u8,
}

impl MatchStartValidator {
    pub fn new(session_id: Uuid, players_per_team: u8) -> Self {
        Self {
            session_id,
            players_per_team,
        }
    }

    pub fn validate_start(
        &self,
        session_teams: &[Team],
        session_matches: &[Match],
        team_a_id: Uuid,
        team_b_id: Uuid,
    ) -> HttpResult<()> {
        let busy_team_ids = Match::busy_team_ids(session_matches);

        for team_id in [team_a_id, team_b_id] {
            self.validate_team_can_start(session_teams, &busy_team_ids, team_id)?;
        }

        Ok(())
    }

    fn validate_team_can_start(
        &self,
        session_teams: &[Team],
        busy_team_ids: &HashSet<Uuid>,
        team_id: Uuid,
    ) -> HttpResult<()> {
        let team = session_teams
            .iter()
            .find(|team| *team.id() == team_id && *team.session_id() == self.session_id)
            .ok_or_else(|| Box::new(HttpError::not_found("Team", team_id)))?;

        if team.is_disbanded() {
            return Err(Box::new(HttpError::conflict(format!(
                "Team {team_id} has been disbanded and cannot start a match"
            ))));
        }

        if !team.is_complete(self.players_per_team) {
            return Err(Box::new(HttpError::conflict(format!(
                "Team {team_id} is still waiting on a partner and cannot start a match"
            ))));
        }

        if busy_team_ids.contains(&team_id) {
            return Err(Box::new(HttpError::conflict(format!(
                "Team {team_id} is already playing an in-progress match"
            ))));
        }

        Ok(())
    }
}

getters! {
    Match {
        id: Uuid,
        session_id: Uuid,
        court: u8,
        team_a_id: Uuid,
        team_b_id: Uuid,
        winner_team_id: Option<Uuid>,
        started_at: DateTime<Utc>,
        played_at: Option<DateTime<Utc>>,
    }
}

impl From<&sqlx::postgres::PgRow> for Match {
    fn from(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;

        Self {
            id: row.get("id"),
            session_id: row.get("session_id"),
            court: row.get::<i16, _>("court") as u8,
            team_a_id: row.get("team_a_id"),
            team_b_id: row.get("team_b_id"),
            winner_team_id: row.get("winner_team_id"),
            started_at: row.get("started_at"),
            played_at: row.get("played_at"),
        }
    }
}

#[cfg(test)]
mod tests {
    use http_error::HttpErrorKind;

    use super::*;

    fn new_match() -> Match {
        Match::new(Uuid::new_v4(), 1, Uuid::new_v4(), Uuid::new_v4()).unwrap()
    }

    #[test]
    fn test_new_rejects_team_playing_against_itself() {
        let team_id = Uuid::new_v4();

        let err = Match::new(Uuid::new_v4(), 1, team_id, team_id).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_match_starts_unfinished() {
        let match_ = new_match();

        assert!(!match_.is_finished());
        assert!(match_.winner_team_id().is_none());
        assert!(match_.played_at().is_none());
    }

    #[test]
    fn test_finish_records_winner_and_played_at() {
        let mut match_ = new_match();
        let winner = *match_.team_a_id();

        match_.finish(winner).unwrap();

        assert!(match_.is_finished());
        assert_eq!(match_.winner_team_id(), &Some(winner));
        assert!(match_.played_at().is_some());
    }

    #[test]
    fn test_finish_rejects_winner_outside_the_match() {
        let mut match_ = new_match();

        let err = match_.finish(Uuid::new_v4()).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_finish_rejects_reporting_result_twice() {
        let mut match_ = new_match();
        let winner = *match_.team_a_id();
        match_.finish(winner).unwrap();

        let err = match_.finish(winner).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_busy_team_ids_only_includes_unfinished_matches() {
        let team_a = Uuid::new_v4();
        let team_b = Uuid::new_v4();
        let team_c = Uuid::new_v4();
        let team_d = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let ongoing = Match::new(session_id, 1, team_a, team_b).unwrap();
        let mut finished = Match::new(session_id, 2, team_c, team_d).unwrap();
        finished.finish(team_c).unwrap();

        let busy = Match::busy_team_ids(&[ongoing, finished]);

        assert!(busy.contains(&team_a));
        assert!(busy.contains(&team_b));
        assert!(!busy.contains(&team_c));
        assert!(!busy.contains(&team_d));
    }

    fn team(session_id: Uuid) -> Team {
        Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()])
    }

    #[test]
    fn test_match_start_validator_accepts_waiting_teams_from_the_same_session() {
        let session_id = Uuid::new_v4();
        let team_a = team(session_id);
        let team_b = team(session_id);
        let team_a_id = *team_a.id();
        let team_b_id = *team_b.id();

        let result = MatchStartValidator::new(session_id, 2).validate_start(
            &[team_a, team_b],
            &[],
            team_a_id,
            team_b_id,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_match_start_validator_rejects_incomplete_team() {
        let session_id = Uuid::new_v4();
        let incomplete_team = Team::new(session_id, vec![Uuid::new_v4()]);
        let team_b = team(session_id);
        let team_a_id = *incomplete_team.id();
        let team_b_id = *team_b.id();

        let err = MatchStartValidator::new(session_id, 2)
            .validate_start(&[incomplete_team, team_b], &[], team_a_id, team_b_id)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_match_start_validator_rejects_team_from_another_session() {
        let session_id = Uuid::new_v4();
        let team_a = team(session_id);
        let team_b = team(Uuid::new_v4());
        let team_a_id = *team_a.id();
        let team_b_id = *team_b.id();

        let err = MatchStartValidator::new(session_id, 2)
            .validate_start(&[team_a, team_b], &[], team_a_id, team_b_id)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::NotFound);
    }

    #[test]
    fn test_match_start_validator_rejects_disbanded_team() {
        let session_id = Uuid::new_v4();
        let mut team_a = team(session_id);
        team_a.disband();
        let team_b = team(session_id);
        let team_a_id = *team_a.id();
        let team_b_id = *team_b.id();

        let err = MatchStartValidator::new(session_id, 2)
            .validate_start(&[team_a, team_b], &[], team_a_id, team_b_id)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_match_start_validator_rejects_team_already_in_an_ongoing_match() {
        let session_id = Uuid::new_v4();
        let team_a = team(session_id);
        let team_b = team(session_id);
        let team_c = team(session_id);
        let team_a_id = *team_a.id();
        let team_b_id = *team_b.id();
        let team_c_id = *team_c.id();

        let ongoing = Match::new(session_id, 1, team_a_id, team_b_id).unwrap();

        let err = MatchStartValidator::new(session_id, 2)
            .validate_start(&[team_a, team_b, team_c], &[ongoing], team_a_id, team_c_id)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }
}
