use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::volleyball::{
    domain::player::{Player, PlayerFilters},
    handler::player::use_cases::{CreatePlayerRequest, UpdatePlayerRequest},
    repository::player::DynPlayerRepository,
};

#[async_trait]
pub trait PlayerHandler {
    async fn create_player(&self, request: CreatePlayerRequest) -> HttpResult<Player>;

    async fn list_players(&self, filters: PlayerFilters) -> HttpResult<Vec<Player>>;

    async fn update_player(
        &self,
        player_id: Uuid,
        request: UpdatePlayerRequest,
    ) -> HttpResult<Player>;

    async fn delete_player(&self, player_id: Uuid) -> HttpResult<()>;
}

pub type DynPlayerHandler = dyn PlayerHandler + Send + Sync;

#[derive(Clone)]
pub struct PlayerHandlerImpl {
    pub player_repository: Arc<DynPlayerRepository>,
}

#[async_trait]
impl PlayerHandler for PlayerHandlerImpl {
    async fn create_player(&self, request: CreatePlayerRequest) -> HttpResult<Player> {
        let player = Player::new(request.name);
        self.player_repository.insert(player).await
    }

    async fn list_players(&self, filters: PlayerFilters) -> HttpResult<Vec<Player>> {
        self.player_repository.list(&filters).await
    }

    async fn update_player(
        &self,
        player_id: Uuid,
        request: UpdatePlayerRequest,
    ) -> HttpResult<Player> {
        let mut player = self
            .player_repository
            .get_by_id(player_id)
            .await?
            .or_not_found("player", player_id.to_string())?;

        if let Some(name) = request.name {
            player.set_name(name);
        }

        self.player_repository.update(player).await
    }

    async fn delete_player(&self, player_id: Uuid) -> HttpResult<()> {
        let mut player = self
            .player_repository
            .get_by_id(player_id)
            .await?
            .or_not_found("player", player_id.to_string())?;

        player.soft_delete();
        self.player_repository.update(player).await?;

        Ok(())
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreatePlayerRequest {
        pub name: String,
    }

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdatePlayerRequest {
        pub name: Option<String>,
    }
}
