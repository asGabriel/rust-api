use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::matchmaking::domain::player::Player;

#[async_trait]
pub trait PlayerRepository {
    async fn insert(&self, player: Player) -> HttpResult<Player>;

    async fn list(&self) -> HttpResult<Vec<Player>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Player>>;

    async fn update(&self, player: Player) -> HttpResult<Player>;
}

pub type DynPlayerRepository = dyn PlayerRepository + Send + Sync;

pub struct PlayerRepositoryImpl {
    pool: Pool<Postgres>,
}

impl PlayerRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn insert(&self, player: Player) -> HttpResult<Player> {
        let gender: String = (*player.gender()).into();

        let row = sqlx::query(
            r#"
            INSERT INTO matchmaking.player (id, name, gender, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(*player.id())
        .bind(player.name())
        .bind(gender)
        .bind(*player.created_at())
        .bind(*player.updated_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Player::from(&row))
    }

    async fn list(&self) -> HttpResult<Vec<Player>> {
        let rows = sqlx::query("SELECT * FROM matchmaking.player")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(Player::from).collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Player>> {
        let row = sqlx::query("SELECT * FROM matchmaking.player WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.as_ref().map(Player::from))
    }

    async fn update(&self, player: Player) -> HttpResult<Player> {
        let gender: String = (*player.gender()).into();

        let row = sqlx::query(
            r#"
            UPDATE matchmaking.player SET
                name = $2,
                gender = $3,
                updated_at = $4
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(*player.id())
        .bind(player.name())
        .bind(gender)
        .bind(*player.updated_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Player::from(&row))
    }
}
