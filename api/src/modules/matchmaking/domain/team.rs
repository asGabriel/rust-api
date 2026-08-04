use std::collections::HashSet;

use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

use crate::modules::matchmaking::domain::{
    player::{Gender, Player},
    session::GameMode,
};

/// A pair/team formed from the players confirmed in a `Session`,
/// used to compose that day's matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    id: Uuid,
    session_id: Uuid,
    player_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
}

impl Team {
    pub fn new(session_id: Uuid, player_ids: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            player_ids,
            created_at: Utc::now(),
        }
    }
}

getters! {
    Team {
        id: Uuid,
        session_id: Uuid,
        player_ids: Vec<Uuid>,
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

/// Randomly forms teams out of a session's confirmed players, honoring the
/// session's `GameMode` filter. Used for a session's first draw, when there
/// is no history yet to balance teams by; later draws may layer other
/// criteria on top.
pub struct TeamDrawer {
    game_mode: GameMode,
    players_per_team: u8,
}

impl TeamDrawer {
    pub fn new(game_mode: GameMode, players_per_team: u8) -> Self {
        Self {
            game_mode,
            players_per_team,
        }
    }

    /// Returns as many full teams as can be formed from `players`, each a
    /// list of player ids. Players left over (not enough to fill one more
    /// team, or of the gender not needed to complete a mixed team) are
    /// simply not included in any group.
    pub fn draw(&self, players: &[Player]) -> HttpResult<Vec<Vec<Uuid>>> {
        if self.players_per_team == 0 {
            return Err(Box::new(HttpError::bad_request(
                "Session settings must have at least one player per team",
            )));
        }
        self.game_mode
            .validate_players_per_team(self.players_per_team)?;

        match self.game_mode {
            GameMode::Male => Ok(self.draw_single_gender(players, Gender::Male)),
            GameMode::Female => Ok(self.draw_single_gender(players, Gender::Female)),
            GameMode::Mixed => Ok(self.draw_mixed(players)),
        }
    }

    fn draw_single_gender(&self, players: &[Player], gender: Gender) -> Vec<Vec<Uuid>> {
        let mut pool = Self::player_ids_of_gender(players, gender);
        pool.shuffle(&mut thread_rng());

        Self::chunk_into_teams(&pool, self.players_per_team as usize)
    }

    fn draw_mixed(&self, players: &[Player]) -> Vec<Vec<Uuid>> {
        let players_per_gender = (self.players_per_team / 2) as usize;

        let mut males = Self::player_ids_of_gender(players, Gender::Male);
        let mut females = Self::player_ids_of_gender(players, Gender::Female);
        males.shuffle(&mut thread_rng());
        females.shuffle(&mut thread_rng());

        let team_count = (males.len() / players_per_gender).min(females.len() / players_per_gender);

        (0..team_count)
            .map(|i| {
                let start = i * players_per_gender;
                let end = start + players_per_gender;

                let mut team = males[start..end].to_vec();
                team.extend_from_slice(&females[start..end]);
                team
            })
            .collect()
    }

    fn player_ids_of_gender(players: &[Player], gender: Gender) -> Vec<Uuid> {
        players
            .iter()
            .filter(|player| *player.gender() == gender)
            .map(|player| *player.id())
            .collect()
    }

    fn chunk_into_teams(pool: &[Uuid], players_per_team: usize) -> Vec<Vec<Uuid>> {
        pool.chunks_exact(players_per_team)
            .map(|chunk| chunk.to_vec())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use http_error::HttpErrorKind;

    use super::*;

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

    fn player(gender: Gender) -> Player {
        Player::new("Player".to_string(), gender)
    }

    #[test]
    fn test_draw_male_mode_only_pairs_male_players() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Female),
            player(Gender::Female),
        ];
        let male_ids: HashSet<Uuid> = players
            .iter()
            .filter(|player| *player.gender() == Gender::Male)
            .map(|player| *player.id())
            .collect();

        let teams = TeamDrawer::new(GameMode::Male, 2).draw(&players).unwrap();

        assert_eq!(teams.len(), 2);
        for team in teams {
            assert_eq!(team.len(), 2);
            assert!(team.iter().all(|id| male_ids.contains(id)));
        }
    }

    #[test]
    fn test_draw_female_mode_leaves_leftover_player_unassigned() {
        let players = vec![
            player(Gender::Female),
            player(Gender::Female),
            player(Gender::Female),
            player(Gender::Male),
        ];

        let teams = TeamDrawer::new(GameMode::Female, 2).draw(&players).unwrap();

        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].len(), 2);
    }

    #[test]
    fn test_draw_mixed_mode_pairs_one_man_and_one_woman_per_team() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Female),
            player(Gender::Female),
        ];
        let male_ids: HashSet<Uuid> = players
            .iter()
            .filter(|player| *player.gender() == Gender::Male)
            .map(|player| *player.id())
            .collect();
        let female_ids: HashSet<Uuid> = players
            .iter()
            .filter(|player| *player.gender() == Gender::Female)
            .map(|player| *player.id())
            .collect();

        let teams = TeamDrawer::new(GameMode::Mixed, 2).draw(&players).unwrap();

        assert_eq!(teams.len(), 2);
        for team in teams {
            assert_eq!(team.len(), 2);
            assert_eq!(team.iter().filter(|id| male_ids.contains(id)).count(), 1);
            assert_eq!(team.iter().filter(|id| female_ids.contains(id)).count(), 1);
        }
    }

    #[test]
    fn test_draw_mixed_mode_limited_by_smaller_gender_pool() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Female),
        ];

        let teams = TeamDrawer::new(GameMode::Mixed, 2).draw(&players).unwrap();

        assert_eq!(teams.len(), 1);
    }

    #[test]
    fn test_draw_rejects_zero_players_per_team() {
        let err = TeamDrawer::new(GameMode::Male, 0).draw(&[]).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_draw_mixed_rejects_odd_players_per_team() {
        let players = vec![player(Gender::Male), player(Gender::Female)];

        let err = TeamDrawer::new(GameMode::Mixed, 3)
            .draw(&players)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }
}
