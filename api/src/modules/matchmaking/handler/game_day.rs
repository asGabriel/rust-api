use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::game_day::GameDay,
    handler::game_day::use_cases::{CreateGameDayRequest, UpdateGameDayRequest},
    repository::game_day::DynGameDayRepository,
};

#[async_trait]
pub trait GameDayHandler {
    async fn create_game_day(&self, request: CreateGameDayRequest) -> HttpResult<GameDay>;

    async fn list_game_days(&self) -> HttpResult<Vec<GameDay>>;

    async fn get_game_day(&self, id: Uuid) -> HttpResult<GameDay>;

    async fn update_game_day(&self, id: Uuid, request: UpdateGameDayRequest)
        -> HttpResult<GameDay>;
}

pub type DynGameDayHandler = dyn GameDayHandler + Send + Sync;

#[derive(Clone)]
pub struct GameDayHandlerImpl {
    pub game_day_repository: Arc<DynGameDayRepository>,
}

#[async_trait]
impl GameDayHandler for GameDayHandlerImpl {
    async fn create_game_day(&self, request: CreateGameDayRequest) -> HttpResult<GameDay> {
        let game_day = GameDay::new(
            request.date,
            request.settings.unwrap_or_default(),
            request.available_courts,
        );

        self.game_day_repository.insert(game_day).await
    }

    async fn list_game_days(&self) -> HttpResult<Vec<GameDay>> {
        self.game_day_repository.list().await
    }

    async fn get_game_day(&self, id: Uuid) -> HttpResult<GameDay> {
        self.game_day_repository
            .get(&id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("GameDay", id)))
    }

    async fn update_game_day(
        &self,
        id: Uuid,
        request: UpdateGameDayRequest,
    ) -> HttpResult<GameDay> {
        let mut game_day = self.get_game_day(id).await?;

        if let Some(date) = request.date {
            game_day.set_date(date);
        }

        if let Some(settings) = request.settings {
            game_day.set_settings(settings);
        }

        if let Some(available_courts) = request.available_courts {
            game_day.set_available_courts(available_courts);
        }

        if let Some(player_ids) = request.player_ids {
            game_day.set_player_ids(player_ids);
        }

        self.game_day_repository.update(game_day).await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::matchmaking::domain::game_day::GameDaySettings;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateGameDayRequest {
        pub date: NaiveDate,
        pub available_courts: u8,
        pub settings: Option<GameDaySettings>,
    }

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateGameDayRequest {
        pub date: Option<NaiveDate>,
        pub available_courts: Option<u8>,
        pub settings: Option<GameDaySettings>,
        pub player_ids: Option<Vec<Uuid>>,
    }
}
