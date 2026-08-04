use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::team::{Team, TeamDrawer, TeamValidator},
    handler::team::use_cases::CreateTeamRequest,
    repository::{
        player::DynPlayerRepository, session::DynSessionRepository, team::DynTeamRepository,
    },
};

#[async_trait]
pub trait TeamHandler {
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team>;

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;

    /// First draw of teams for a session: random pairing of the session's
    /// confirmed players, honoring the session's `GameMode` filter. Fails if
    /// the session already has teams, since it's meant to initialize them.
    async fn draw_teams(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;
}

pub type DynTeamHandler = dyn TeamHandler + Send + Sync;

#[derive(Clone)]
pub struct TeamHandlerImpl {
    pub team_repository: Arc<DynTeamRepository>,
    pub session_repository: Arc<DynSessionRepository>,
    pub player_repository: Arc<DynPlayerRepository>,
}

#[async_trait]
impl TeamHandler for TeamHandlerImpl {
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team> {
        let existing_teams = self
            .team_repository
            .list_by_session(&request.session_id)
            .await?;

        TeamValidator::new(request.session_id)
            .validate_new_team(&existing_teams, &request.player_ids)?;

        let team = Team::new(request.session_id, request.player_ids);

        self.team_repository.insert(team).await
    }

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        self.team_repository.list_by_session(&session_id).await
    }

    async fn draw_teams(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        let existing_teams = self.team_repository.list_by_session(&session_id).await?;
        if !existing_teams.is_empty() {
            return Err(Box::new(HttpError::conflict(
                "Session already has teams drawn",
            )));
        }

        let session = self
            .session_repository
            .get(&session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", session_id)))?;

        let session_players: Vec<_> = self
            .player_repository
            .list()
            .await?
            .into_iter()
            .filter(|player| session.player_ids().contains(player.id()))
            .collect();

        let drawn_teams =
            TeamDrawer::new(*session.game_mode(), *session.settings().players_per_team())
                .draw(&session_players)?;

        if drawn_teams.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Not enough players to form a team with the session's game mode",
            )));
        }

        let validator = TeamValidator::new(session_id);
        let mut created_teams = Vec::with_capacity(drawn_teams.len());

        for player_ids in drawn_teams {
            validator.validate_new_team(&created_teams, &player_ids)?;

            let team = Team::new(session_id, player_ids);
            created_teams.push(self.team_repository.insert(team).await?);
        }

        Ok(created_teams)
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateTeamRequest {
        pub session_id: Uuid,
        pub player_ids: Vec<Uuid>,
    }
}
