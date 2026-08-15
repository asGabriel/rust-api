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
    /// Set by `Team::with_priority` — jumps this team ahead of every
    /// non-priority team in `TeamQueueManager::next_complete_teams`.
    priority: bool,
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
            priority: false,
            created_at: Utc::now(),
        }
    }

    /// Marks this team to jump the queue: the manual "who plays next"
    /// override for when an operator needs to put a specific pairing on
    /// court next, regardless of how long everyone else has been waiting.
    /// See `TeamQueueManager::next_complete_teams`.
    pub fn with_priority(mut self) -> Self {
        self.priority = true;
        self
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

    pub fn is_priority(&self) -> bool {
        self.priority
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

/// Centralizes the validation rules for forming a `Team` within a `Session`,
/// including the manual (contingency) path: an operator picking specific
/// players to force a team into the queue, regardless of the session's
/// `GameMode`/`ShuffleType` — those only constrain the automated draw and
/// queue rotation, never a manually assembled team.
/// Bound to a `session_id` and that session's confirmed `player_ids` at
/// construction so it always scopes its checks to that session, regardless
/// of what `existing_teams` the caller passes in.
pub struct TeamValidator {
    session_id: Uuid,
    session_player_ids: Vec<Uuid>,
}

impl TeamValidator {
    pub fn new(session_id: Uuid, session_player_ids: Vec<Uuid>) -> Self {
        Self {
            session_id,
            session_player_ids,
        }
    }

    /// A player cannot repeat within the same team, must be a player
    /// confirmed for the session, and cannot be in another active
    /// (non-disbanded) team of the same session at the same time. A
    /// disbanded team's players are free again, so they never block a new
    /// team here.
    pub fn validate_new_team(
        &self,
        existing_teams: &[Team],
        player_ids: &[Uuid],
    ) -> HttpResult<()> {
        Self::reject_duplicate_players_in_team(player_ids)?;
        self.reject_players_not_confirmed_in_session(player_ids)?;
        self.reject_players_already_in_active_team(existing_teams, player_ids)?;

        Ok(())
    }

    /// Validates a manual "priority" team — the operator override to put a
    /// specific pairing on court next (see `Team::with_priority`). Same
    /// duplicate/session-membership checks as `validate_new_team`, but here
    /// a player already in another `Waiting` team is allowed *if* that team
    /// isn't tied up in an in-progress match: pulling them out to jump this
    /// pairing to the front of the queue is exactly what this path is for.
    /// A player in a `Holding` team, or one that's playing right now, can't
    /// be pulled — that would strand a court still being defended or an
    /// in-progress match.
    ///
    /// Returns, for each distinct origin team a player was pulled from, that
    /// team paired with the ids of its now partner-less remaining players —
    /// the caller must disband the origin team and re-slot those remaining
    /// players back into the queue (see `TeamQueueManager::release_players`).
    pub fn validate_priority_team(
        &self,
        existing_teams: &[Team],
        busy_team_ids: &HashSet<Uuid>,
        player_ids: &[Uuid],
    ) -> HttpResult<Vec<(Team, Vec<Uuid>)>> {
        Self::reject_duplicate_players_in_team(player_ids)?;
        self.reject_players_not_confirmed_in_session(player_ids)?;
        self.collect_broken_origin_teams(existing_teams, busy_team_ids, player_ids)
    }

    fn collect_broken_origin_teams(
        &self,
        existing_teams: &[Team],
        busy_team_ids: &HashSet<Uuid>,
        player_ids: &[Uuid],
    ) -> HttpResult<Vec<(Team, Vec<Uuid>)>> {
        let mut broken: Vec<(Team, Vec<Uuid>)> = Vec::new();

        for player_id in player_ids {
            let Some(owner) = existing_teams.iter().find(|team| {
                team.session_id() == &self.session_id
                    && !team.is_disbanded()
                    && team.player_ids().contains(player_id)
            }) else {
                continue;
            };

            if !owner.is_waiting() || busy_team_ids.contains(owner.id()) {
                return Err(Box::new(HttpError::conflict(format!(
                    "Player {player_id} is in a team that can't be broken up right now"
                ))));
            }

            match broken.iter_mut().find(|(team, _)| team.id() == owner.id()) {
                Some((_, remaining)) => remaining.retain(|id| id != player_id),
                None => {
                    let remaining = owner
                        .player_ids()
                        .iter()
                        .copied()
                        .filter(|id| id != player_id)
                        .collect();
                    broken.push((owner.clone(), remaining));
                }
            }
        }

        Ok(broken)
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

    fn reject_players_not_confirmed_in_session(&self, player_ids: &[Uuid]) -> HttpResult<()> {
        if let Some(player_id) = player_ids
            .iter()
            .find(|player_id| !self.session_player_ids.contains(player_id))
        {
            return Err(Box::new(HttpError::bad_request(format!(
                "Player {player_id} is not a confirmed player of this session"
            ))));
        }

        Ok(())
    }

    fn reject_players_already_in_active_team(
        &self,
        existing_teams: &[Team],
        player_ids: &[Uuid],
    ) -> HttpResult<()> {
        let players_in_session: HashSet<&Uuid> = existing_teams
            .iter()
            .filter(|team| team.session_id() == &self.session_id && !team.is_disbanded())
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
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let validator = TeamValidator::new(Uuid::new_v4(), player_ids.clone());

        assert!(validator.validate_new_team(&[], &player_ids).is_ok());
    }

    #[test]
    fn test_validate_new_team_with_single_player() {
        let player_ids = vec![Uuid::new_v4()];
        let validator = TeamValidator::new(Uuid::new_v4(), player_ids.clone());

        assert!(validator.validate_new_team(&[], &player_ids).is_ok());
    }

    #[test]
    fn test_validate_new_team_rejects_duplicated_player_in_same_team() {
        let player_id = Uuid::new_v4();
        let player_ids = vec![player_id, player_id];
        let validator = TeamValidator::new(Uuid::new_v4(), player_ids.clone());

        let err = validator.validate_new_team(&[], &player_ids).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_validate_new_team_rejects_player_not_confirmed_in_session() {
        let session_id = Uuid::new_v4();
        let confirmed_player = Uuid::new_v4();
        let outsider_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id, vec![confirmed_player]);
        let player_ids = vec![confirmed_player, outsider_player];

        let err = validator.validate_new_team(&[], &player_ids).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_validate_new_team_rejects_player_already_in_another_team_of_same_session() {
        let session_id = Uuid::new_v4();
        let repeated_player = Uuid::new_v4();
        let new_player = Uuid::new_v4();

        let validator = TeamValidator::new(
            session_id,
            vec![repeated_player, Uuid::new_v4(), new_player],
        );
        let existing_team = Team::new(session_id, vec![repeated_player, Uuid::new_v4()]);
        let player_ids = vec![repeated_player, new_player];

        let err = validator
            .validate_new_team(std::slice::from_ref(&existing_team), &player_ids)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    /// A disbanded team's players are free again — they must not block a
    /// manually created team, which is exactly the contingency scenario a
    /// manual team is meant to unblock (e.g. re-forming a team right after
    /// its previous one lost and was disbanded).
    #[test]
    fn test_validate_new_team_ignores_players_from_a_disbanded_team() {
        let session_id = Uuid::new_v4();
        let freed_player = Uuid::new_v4();
        let partner = Uuid::new_v4();

        let validator = TeamValidator::new(session_id, vec![freed_player, partner]);
        let mut disbanded_team = Team::new(session_id, vec![freed_player, Uuid::new_v4()]);
        disbanded_team.disband();
        let player_ids = vec![freed_player, partner];

        let result =
            validator.validate_new_team(std::slice::from_ref(&disbanded_team), &player_ids);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_new_team_allows_when_no_players_overlap_with_existing_team() {
        let session_id = Uuid::new_v4();
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let existing_team = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);
        let validator = TeamValidator::new(session_id, player_ids.clone());

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
        let player_ids = vec![shared_player, Uuid::new_v4()];

        let validator = TeamValidator::new(session_id, player_ids.clone());
        let team_from_other_session =
            Team::new(other_session_id, vec![shared_player, Uuid::new_v4()]);

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
        let new_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id, vec![repeated_player, new_player]);
        let team_in_same_session = Team::new(session_id, vec![repeated_player, Uuid::new_v4()]);
        let team_in_other_session =
            Team::new(other_session_id, vec![repeated_player, Uuid::new_v4()]);
        let player_ids = vec![repeated_player, new_player];

        let err = validator
            .validate_new_team(&[team_in_same_session, team_in_other_session], &player_ids)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_with_priority_marks_the_team_as_priority() {
        let team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]).with_priority();

        assert!(team.is_priority());
    }

    #[test]
    fn test_validate_priority_team_allows_a_player_free_in_the_session() {
        let session_id = Uuid::new_v4();
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let validator = TeamValidator::new(session_id, player_ids.clone());

        let broken = validator
            .validate_priority_team(&[], &HashSet::new(), &player_ids)
            .unwrap();

        assert!(broken.is_empty());
    }

    /// The whole point of the priority path: pulling one player out of a
    /// `Waiting` team that isn't mid-match must report that team so the
    /// caller can disband it and re-slot its now partner-less teammate.
    #[test]
    fn test_validate_priority_team_reports_the_origin_team_of_a_stolen_player() {
        let session_id = Uuid::new_v4();
        let stolen_player = Uuid::new_v4();
        let stranded_partner = Uuid::new_v4();
        let new_partner = Uuid::new_v4();

        let origin_team = Team::new(session_id, vec![stolen_player, stranded_partner]);
        let origin_team_id = *origin_team.id();
        let validator = TeamValidator::new(
            session_id,
            vec![stolen_player, stranded_partner, new_partner],
        );
        let player_ids = vec![stolen_player, new_partner];

        let broken = validator
            .validate_priority_team(std::slice::from_ref(&origin_team), &HashSet::new(), &player_ids)
            .unwrap();

        assert_eq!(broken.len(), 1);
        let (reported_team, remaining) = &broken[0];
        assert_eq!(reported_team.id(), &origin_team_id);
        assert_eq!(remaining, &vec![stranded_partner]);
    }

    /// Stealing both members of a pair must report that origin team with no
    /// remaining players — nothing left to re-slot, just disband it.
    #[test]
    fn test_validate_priority_team_reports_empty_remaining_when_both_players_are_stolen() {
        let session_id = Uuid::new_v4();
        let player_a = Uuid::new_v4();
        let player_b = Uuid::new_v4();

        let origin_team = Team::new(session_id, vec![player_a, player_b]);
        let validator = TeamValidator::new(session_id, vec![player_a, player_b]);

        let broken = validator
            .validate_priority_team(
                std::slice::from_ref(&origin_team),
                &HashSet::new(),
                &[player_a, player_b],
            )
            .unwrap();

        assert_eq!(broken.len(), 1);
        assert!(broken[0].1.is_empty());
    }

    #[test]
    fn test_validate_priority_team_rejects_a_player_from_a_holding_team() {
        let session_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let new_partner = Uuid::new_v4();
        let mut holding_team = Team::new(session_id, vec![player_id, Uuid::new_v4()]);
        holding_team.register_win();
        let validator = TeamValidator::new(session_id, vec![player_id, new_partner]);

        let err = validator
            .validate_priority_team(
                std::slice::from_ref(&holding_team),
                &HashSet::new(),
                &[player_id, new_partner],
            )
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_validate_priority_team_rejects_a_player_from_a_busy_team() {
        let session_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let new_partner = Uuid::new_v4();
        let busy_team = Team::new(session_id, vec![player_id, Uuid::new_v4()]);
        let busy_team_id = *busy_team.id();
        let validator = TeamValidator::new(session_id, vec![player_id, new_partner]);

        let err = validator
            .validate_priority_team(
                std::slice::from_ref(&busy_team),
                &HashSet::from([busy_team_id]),
                &[player_id, new_partner],
            )
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_validate_priority_team_ignores_a_disbanded_teammate() {
        let session_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let new_partner = Uuid::new_v4();
        let mut disbanded_team = Team::new(session_id, vec![player_id, Uuid::new_v4()]);
        disbanded_team.disband();
        let validator = TeamValidator::new(session_id, vec![player_id, new_partner]);

        let broken = validator
            .validate_priority_team(
                std::slice::from_ref(&disbanded_team),
                &HashSet::new(),
                &[player_id, new_partner],
            )
            .unwrap();

        assert!(broken.is_empty());
    }
}
