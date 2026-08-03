use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::matches::Match, handler::matches::use_cases::CreateMatchRequest,
    repository::matches::DynMatchRepository,
};

#[async_trait]
pub trait MatchHandler {
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<Match>;

    async fn list_matches_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Match>>;
}

pub type DynMatchHandler = dyn MatchHandler + Send + Sync;

#[derive(Clone)]
pub struct MatchHandlerImpl {
    pub match_repository: Arc<DynMatchRepository>,
}

#[async_trait]
impl MatchHandler for MatchHandlerImpl {
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<Match> {
        let match_ = Match::new(
            request.session_id,
            request.court,
            request.team_a_id,
            request.team_b_id,
            request.winner_team_id,
        );

        self.match_repository.insert(match_).await
    }

    async fn list_matches_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Match>> {
        self.match_repository.list_by_session(&session_id).await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateMatchRequest {
        pub session_id: Uuid,
        pub court: u8,
        pub team_a_id: Uuid,
        pub team_b_id: Uuid,
        pub winner_team_id: Uuid,
    }
}
