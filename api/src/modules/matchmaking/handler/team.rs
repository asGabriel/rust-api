use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::team::Team, handler::team::use_cases::CreateTeamRequest,
    repository::team::DynTeamRepository,
};

#[async_trait]
pub trait TeamHandler {
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team>;

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;
}

pub type DynTeamHandler = dyn TeamHandler + Send + Sync;

#[derive(Clone)]
pub struct TeamHandlerImpl {
    pub team_repository: Arc<DynTeamRepository>,
}

#[async_trait]
impl TeamHandler for TeamHandlerImpl {
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team> {
        let team = Team::new(request.session_id, request.player_ids);

        self.team_repository.insert(team).await
    }

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        self.team_repository.list_by_session(&session_id).await
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
