use std::collections::HashSet;

use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// A team's place in the session's rotation: `Waiting` in the queue to be
/// called up to a court, `Holding` a court after a win (until it either
/// loses or hits the consecutive-win cap), or `Disbanded` once it's done,
/// freeing its players back into the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeamStatus {
    Waiting,
    Holding,
    Disbanded,
}

/// A pair/team formed from the players confirmed in a `Session`,
/// used to compose that day's matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    id: Uuid,
    session_id: Uuid,
    player_ids: Vec<Uuid>,
    status: TeamStatus,
    consecutive_wins: u8,
    created_at: DateTime<Utc>,
}

impl Team {
    /// A team is rotated out after this many consecutive wins on the same
    /// court, so other waiting teams get a turn.
    const MAX_CONSECUTIVE_WINS: u8 = 2;

    pub fn new(session_id: Uuid, player_ids: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            player_ids,
            status: TeamStatus::Waiting,
            consecutive_wins: 0,
            created_at: Utc::now(),
        }
    }

    pub fn is_waiting(&self) -> bool {
        self.status == TeamStatus::Waiting
    }

    pub fn is_holding(&self) -> bool {
        self.status == TeamStatus::Holding
    }

    pub fn is_disbanded(&self) -> bool {
        self.status == TeamStatus::Disbanded
    }

    /// Whether this team has as many players as a full team needs.
    pub fn is_complete(&self, players_per_team: u8) -> bool {
        self.player_ids.len() >= players_per_team.into()
    }

    /// Frees this team's players back into the queue: it's done, whether it
    /// just lost or hit the consecutive-win cap. Idempotent.
    pub fn disband(&mut self) {
        self.status = TeamStatus::Disbanded;
    }

    /// Adds a freed player to this (presumably incomplete) waiting team,
    /// completing it or extending how many players it still needs.
    pub fn add_player(&mut self, player_id: Uuid) {
        self.player_ids.push(player_id);
    }

    /// Records a win. A team keeps holding its court after a win, but only
    /// up to `MAX_CONSECUTIVE_WINS` in a row — past that, it's disbanded
    /// like a loss would be, so other waiting teams get a turn.
    pub fn register_win(&mut self) {
        self.consecutive_wins += 1;

        if self.consecutive_wins >= Self::MAX_CONSECUTIVE_WINS {
            self.disband();
        } else {
            self.status = TeamStatus::Holding;
        }
    }
}

getters! {
    Team {
        id: Uuid,
        session_id: Uuid,
        player_ids: Vec<Uuid>,
        status: TeamStatus,
        consecutive_wins: u8,
        created_at: DateTime<Utc>,
    }
}

/// Centralizes the validation rules for forming a `Team` within a `Session`.
/// Bound to a `session_id` at construction so it always scopes the
/// "player already in another team" check to that session, regardless of
/// what `existing_teams` the caller passes in.
pub struct TeamValidator {
    session_id: Uuid,
}

impl TeamValidator {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }

    /// A player cannot repeat within the same team, nor be in two teams of
    /// the same session at the same time.
    pub fn validate_new_team(
        &self,
        existing_teams: &[Team],
        player_ids: &[Uuid],
    ) -> HttpResult<()> {
        Self::reject_duplicate_players_in_team(player_ids)?;
        self.reject_players_already_in_session(existing_teams, player_ids)?;

        Ok(())
    }

    fn reject_duplicate_players_in_team(player_ids: &[Uuid]) -> HttpResult<()> {
        let mut seen = HashSet::with_capacity(player_ids.len());

        for player_id in player_ids {
            if !seen.insert(player_id) {
                return Err(Box::new(HttpError::bad_request(format!(
                    "Player {player_id} is duplicated in the same team"
                ))));
            }
        }

        Ok(())
    }

    fn reject_players_already_in_session(
        &self,
        existing_teams: &[Team],
        player_ids: &[Uuid],
    ) -> HttpResult<()> {
        let players_in_session: HashSet<&Uuid> = existing_teams
            .iter()
            .filter(|team| team.session_id() == &self.session_id)
            .flat_map(|team| team.player_ids())
            .collect();

        if let Some(player_id) = player_ids
            .iter()
            .find(|player_id| players_in_session.contains(player_id))
        {
            return Err(Box::new(HttpError::conflict(format!(
                "Player {player_id} is already in another team for this session"
            ))));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use http_error::HttpErrorKind;

    use super::*;

    #[test]
    fn test_new_team_starts_waiting_with_no_wins() {
        let team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        assert!(team.is_waiting());
        assert_eq!(*team.consecutive_wins(), 0);
    }

    #[test]
    fn test_register_win_once_holds_the_court() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        team.register_win();

        assert!(team.is_holding());
        assert_eq!(*team.consecutive_wins(), 1);
    }

    #[test]
    fn test_register_win_twice_in_a_row_disbands_the_team() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        team.register_win();
        team.register_win();

        assert!(team.is_disbanded());
        assert_eq!(*team.consecutive_wins(), 2);
    }

    #[test]
    fn test_disband_is_idempotent() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        team.disband();
        team.disband();

        assert!(team.is_disbanded());
    }

    #[test]
    fn test_is_complete_checks_player_count_against_players_per_team() {
        let team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4()]);

        assert!(!team.is_complete(2));

        let full_team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        assert!(full_team.is_complete(2));
    }

    #[test]
    fn test_add_player_extends_the_team() {
        let player_id = Uuid::new_v4();
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4()]);

        team.add_player(player_id);

        assert!(team.player_ids().contains(&player_id));
        assert!(team.is_complete(2));
    }

    #[test]
    fn test_validate_new_team_with_no_existing_teams() {
        let validator = TeamValidator::new(Uuid::new_v4());
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];

        assert!(validator.validate_new_team(&[], &player_ids).is_ok());
    }

    #[test]
    fn test_validate_new_team_with_single_player() {
        let validator = TeamValidator::new(Uuid::new_v4());
        let player_ids = vec![Uuid::new_v4()];

        assert!(validator.validate_new_team(&[], &player_ids).is_ok());
    }

    #[test]
    fn test_validate_new_team_rejects_duplicated_player_in_same_team() {
        let validator = TeamValidator::new(Uuid::new_v4());
        let player_id = Uuid::new_v4();
        let player_ids = vec![player_id, player_id];

        let err = validator.validate_new_team(&[], &player_ids).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_validate_new_team_rejects_player_already_in_another_team_of_same_session() {
        let session_id = Uuid::new_v4();
        let repeated_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id);
        let existing_team = Team::new(session_id, vec![repeated_player, Uuid::new_v4()]);
        let player_ids = vec![repeated_player, Uuid::new_v4()];

        let err = validator
            .validate_new_team(std::slice::from_ref(&existing_team), &player_ids)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_validate_new_team_allows_when_no_players_overlap_with_existing_team() {
        let session_id = Uuid::new_v4();
        let validator = TeamValidator::new(session_id);
        let existing_team = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];

        let result = validator.validate_new_team(std::slice::from_ref(&existing_team), &player_ids);

        assert!(result.is_ok());
    }

    /// The validator scopes the check to its own `session_id`, so a team
    /// belonging to a different session must not block a shared player,
    /// even if the caller passes it in `existing_teams` by mistake.
    #[test]
    fn test_validate_new_team_ignores_teams_from_other_sessions() {
        let session_id = Uuid::new_v4();
        let other_session_id = Uuid::new_v4();
        let shared_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id);
        let team_from_other_session =
            Team::new(other_session_id, vec![shared_player, Uuid::new_v4()]);
        let player_ids = vec![shared_player, Uuid::new_v4()];

        let result = validator
            .validate_new_team(std::slice::from_ref(&team_from_other_session), &player_ids);

        assert!(result.is_ok());
    }

    /// With a mixed list, the session filter must reject the overlap coming
    /// from the same-session team and ignore the other-session one.
    #[test]
    fn test_validate_new_team_filters_by_session_in_a_mixed_team_list() {
        let session_id = Uuid::new_v4();
        let other_session_id = Uuid::new_v4();
        let repeated_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id);
        let team_in_same_session = Team::new(session_id, vec![repeated_player, Uuid::new_v4()]);
        let team_in_other_session =
            Team::new(other_session_id, vec![repeated_player, Uuid::new_v4()]);
        let player_ids = vec![repeated_player, Uuid::new_v4()];

        let err = validator
            .validate_new_team(&[team_in_same_session, team_in_other_session], &player_ids)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }
}
