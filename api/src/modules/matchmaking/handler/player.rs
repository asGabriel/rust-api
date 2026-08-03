use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::player::Player,
    handler::player::use_cases::{CreatePlayerRequest, UpdatePlayerRequest},
    repository::player::DynPlayerRepository,
};

#[async_trait]
pub trait PlayerHandler {
    async fn create_player(&self, request: CreatePlayerRequest) -> HttpResult<Player>;

    async fn list_players(&self) -> HttpResult<Vec<Player>>;

    async fn update_player(&self, id: Uuid, request: UpdatePlayerRequest) -> HttpResult<Player>;
}

pub type DynPlayerHandler = dyn PlayerHandler + Send + Sync;

#[derive(Clone)]
pub struct PlayerHandlerImpl {
    pub player_repository: Arc<DynPlayerRepository>,
}

#[async_trait]
impl PlayerHandler for PlayerHandlerImpl {
    async fn create_player(&self, request: CreatePlayerRequest) -> HttpResult<Player> {
        let player = Player::new(request.name, request.gender);

        self.player_repository.insert(player).await
    }

    async fn list_players(&self) -> HttpResult<Vec<Player>> {
        self.player_repository.list().await
    }

    async fn update_player(&self, id: Uuid, request: UpdatePlayerRequest) -> HttpResult<Player> {
        let mut player = self
            .player_repository
            .get(&id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Player", id)))?;

        if let Some(name) = request.name {
            player.set_name(name);
        }

        if let Some(gender) = request.gender {
            player.set_gender(gender);
        }

        self.player_repository.update(player).await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};

    use crate::modules::matchmaking::domain::player::Gender;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreatePlayerRequest {
        pub name: String,
        pub gender: Gender,
    }

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdatePlayerRequest {
        pub name: Option<String>,
        pub gender: Option<Gender>,
    }
}
