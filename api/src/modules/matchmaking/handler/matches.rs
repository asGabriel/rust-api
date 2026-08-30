use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::matches::{Match, MatchStartValidator},
    handler::{
        matches::use_cases::{
            CreateMatchRequest, ReportMatchResultRequest, ReportMatchResultResponse,
        },
        team::DynTeamHandler,
    },
    repository::{matches::DynMatchRepository, session::DynSessionRepository},
};

#[async_trait]
pub trait MatchHandler {
    /// Starts a match on a court: assigns the two teams facing off, with no
    /// result yet. Used to open a court for the first time — once it's
    /// occupied, further matches on it are created automatically as results
    /// come in (see `report_match_result`).
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<Match>;

    async fn list_matches_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Match>>;

    /// Reports the result of a match in progress on a court and applies it to
    /// the queue: the loser's players go back on the list, the winner keeps
    /// (or loses) the court per the consecutive-win cap, and every idle court
    /// gets a challenger `Draft` suggested from the queue. Does NOT start the
    /// next match — the response carries the suggestions for the operator to
    /// confirm via `POST /matchmaking/matches/`.
    async fn report_match_result(
        &self,
        match_id: Uuid,
        request: ReportMatchResultRequest,
    ) -> HttpResult<ReportMatchResultResponse>;
}

pub type DynMatchHandler = dyn MatchHandler + Send + Sync;

#[derive(Clone)]
pub struct MatchHandlerImpl {
    pub match_repository: Arc<DynMatchRepository>,
    pub session_repository: Arc<DynSessionRepository>,
    pub team_handler: Arc<DynTeamHandler>,
}

#[async_trait]
impl MatchHandler for MatchHandlerImpl {
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<Match> {
        let session = self
            .session_repository
            .get(&request.session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", request.session_id)))?;
        let session_teams = self
            .team_handler
            .list_teams_by_session(request.session_id)
            .await?;
        let session_matches = self
            .match_repository
            .list_by_session(&request.session_id)
            .await?;

        MatchStartValidator::new(
            request.session_id,
            session.player_ids().clone(),
            *session.settings().players_per_team(),
        )
        .validate_start(
            &session_teams,
            &session_matches,
            request.team_a_id,
            request.team_b_id,
        )?;

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
    ) -> HttpResult<ReportMatchResultResponse> {
        let mut match_ = self
            .match_repository
            .get(&match_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Match", match_id)))?;

        match_.finish(request.winner_team_id)?;
        let finished_match = self.match_repository.update(match_).await?;

        let loser_team_id = if request.winner_team_id == *finished_match.team_a_id() {
            *finished_match.team_b_id()
        } else {
            *finished_match.team_a_id()
        };

        let rotation = self
            .team_handler
            .resolve_match_result(
                *finished_match.session_id(),
                request.winner_team_id,
                loser_team_id,
            )
            .await?;

        Ok(ReportMatchResultResponse {
            match_: finished_match,
            courts: rotation.courts,
        })
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::matchmaking::{
        domain::matches::Match, handler::team::use_cases::CourtSuggestion,
    };

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

    /// The finished match, plus what the result did to each idle court's
    /// queue: the `Holding` team (if any) and the challenger `Draft`(s) the
    /// operator should confirm/edit/start. Nothing is auto-started.
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ReportMatchResultResponse {
        #[serde(rename = "match")]
        pub match_: Match,
        pub courts: Vec<CourtSuggestion>,
    }
}
