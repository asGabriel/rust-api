use async_trait::async_trait;
use chrono::{DateTime, Utc};
use http_error::{ext::OptionHttpExt, HttpResult};
use sqlx::postgres::PgRow;
use sqlx::{Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::modules::volleyball::domain::player::{Player, PlayerFilters};

#[async_trait]
pub trait PlayerRepository {
    async fn insert(&self, player: Player) -> HttpResult<Player>;

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Player>>;

    async fn list(&self, filters: &PlayerFilters) -> HttpResult<Vec<Player>>;

    async fn update(&self, player: Player) -> HttpResult<Player>;
}

pub type DynPlayerRepository = dyn PlayerRepository + Send + Sync;

#[derive(Clone)]
pub struct PlayerRepositoryImpl {
    pool: Pool<Postgres>,
}

impl PlayerRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

fn player_from_row(row: &PgRow) -> Player {
    let id: Uuid = row.get("id");
    let name: String = row.get("name");
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: Option<DateTime<Utc>> = row.get("updated_at");
    let deleted_at: Option<DateTime<Utc>> = row.get("deleted_at");

    Player::from_row(id, name, created_at, updated_at, deleted_at)
}

#[async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn insert(&self, player: Player) -> HttpResult<Player> {
        let row = sqlx::query(
            r#"
            INSERT INTO volleyball.player (id, name, created_at, updated_at, deleted_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(player.id())
        .bind(player.name())
        .bind(player.created_at())
        .bind(player.updated_at())
        .bind(player.deleted_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(player_from_row(&row))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Player>> {
        let row =
            sqlx::query(r#"SELECT * FROM volleyball.player WHERE id = $1 AND deleted_at IS NULL"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| player_from_row(&r)))
    }

    async fn update(&self, player: Player) -> HttpResult<Player> {
        let row = sqlx::query(
            r#"
            UPDATE volleyball.player
            SET name = $2, updated_at = $3, deleted_at = $4
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(player.id())
        .bind(player.name())
        .bind(player.updated_at())
        .bind(player.deleted_at())
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("player", player.id().to_string())?;

        Ok(player_from_row(&row))
    }

    async fn list(&self, filters: &PlayerFilters) -> HttpResult<Vec<Player>> {
        let mut builder =
            QueryBuilder::new("SELECT * FROM volleyball.player WHERE deleted_at IS NULL");

        if let Some(ids) = filters.ids() {
            builder.push(" AND id = ANY(");
            builder.push_bind(ids);
            builder.push(")");
        }

        if let Some(name) = filters.name() {
            builder.push(" AND name ILIKE ");
            builder.push_bind(format!("%{}%", name));
        }

        builder.push(" ORDER BY name ASC");

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows.iter().map(player_from_row).collect())
    }
}
