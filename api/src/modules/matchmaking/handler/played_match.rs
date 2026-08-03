use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::played_match::PlayedMatch, handler::played_match::use_cases::CreateMatchRequest,
    repository::played_match::DynPlayedMatchRepository,
};

#[async_trait]
pub trait PlayedMatchHandler {
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<PlayedMatch>;

    async fn list_matches_by_game_day(&self, game_day_id: Uuid) -> HttpResult<Vec<PlayedMatch>>;
}

pub type DynPlayedMatchHandler = dyn PlayedMatchHandler + Send + Sync;

#[derive(Clone)]
pub struct PlayedMatchHandlerImpl {
    pub played_match_repository: Arc<DynPlayedMatchRepository>,
}

#[async_trait]
impl PlayedMatchHandler for PlayedMatchHandlerImpl {
    async fn create_match(&self, request: CreateMatchRequest) -> HttpResult<PlayedMatch> {
        let played_match = PlayedMatch::new(
            request.game_day_id,
            request.court,
            request.team_a_id,
            request.team_b_id,
            request.winner_team_id,
        );

        self.played_match_repository.insert(played_match).await
    }

    async fn list_matches_by_game_day(&self, game_day_id: Uuid) -> HttpResult<Vec<PlayedMatch>> {
        self.played_match_repository
            .list_by_game_day(&game_day_id)
            .await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateMatchRequest {
        pub game_day_id: Uuid,
        pub court: u8,
        pub team_a_id: Uuid,
        pub team_b_id: Uuid,
        pub winner_team_id: Uuid,
    }
}
