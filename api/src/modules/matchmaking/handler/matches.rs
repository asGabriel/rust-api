use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::matches::Match,
    handler::matches::use_cases::{CreateMatchRequest, ReportMatchResultRequest},
    repository::matches::DynMatchRepository,
};

#[async_trait]
pub trait MatchHandler {
    /// Starts a match on a court: assigns the two teams facing off, with no
    /// result yet.
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<Match>;

    async fn list_matches_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Match>>;

    /// Reports the result of a match that's in progress on a court.
    async fn report_match_result(
        &self,
        match_id: Uuid,
        request: ReportMatchResultRequest,
    ) -> HttpResult<Match>;
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
        )?;

        self.match_repository.insert(match_).await
    }

    async fn list_matches_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Match>> {
        self.match_repository.list_by_session(&session_id).await
    }

    async fn report_match_result(
        &self,
        match_id: Uuid,
        request: ReportMatchResultRequest,
    ) -> HttpResult<Match> {
        let mut match_ = self
            .match_repository
            .get(&match_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Match", match_id)))?;

        match_.finish(request.winner_team_id)?;

        self.match_repository.update(match_).await
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
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ReportMatchResultRequest {
        pub winner_team_id: Uuid,
    }
}
