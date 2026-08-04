use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

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

    /// Records the result of this in-progress match. Fails if a result was
    /// already reported, or if `winner_team_id` isn't one of the two teams
    /// that played it.
    pub fn finish(&mut self, winner_team_id: Uuid) -> HttpResult<()> {
        if self.is_finished() {
            return Err(Box::new(HttpError::conflict(
                "Match already has a recorded result",
            )));
        }

        if winner_team_id != self.team_a_id && winner_team_id != self.team_b_id {
            return Err(Box::new(HttpError::bad_request(
                "winner_team_id must be one of the match's two teams",
            )));
        }

        self.winner_team_id = Some(winner_team_id);
        self.played_at = Some(Utc::now());

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
}
