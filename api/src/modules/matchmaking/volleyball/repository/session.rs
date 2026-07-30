use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use http_error::{ext::OptionHttpExt, HttpResult};
use sqlx::postgres::PgRow;
use sqlx::{Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::modules::matchmaking::volleyball::domain::{
    player::Player,
    session::{Session, SessionFilters, SessionStatus},
};

#[async_trait]
pub trait SessionRepository {
    async fn insert(&self, session: Session) -> HttpResult<Session>;

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Session>>;

    async fn list(&self, filters: &SessionFilters) -> HttpResult<Vec<Session>>;

    async fn update(&self, session: Session) -> HttpResult<Session>;

    async fn add_roster_players(&self, session_id: Uuid, player_ids: &[Uuid]) -> HttpResult<()>;

    async fn list_roster_player_ids(&self, session_id: Uuid) -> HttpResult<Vec<Uuid>>;

    async fn list_roster_players(&self, session_id: Uuid) -> HttpResult<Vec<Player>>;
}

pub type DynSessionRepository = dyn SessionRepository + Send + Sync;

#[derive(Clone)]
pub struct SessionRepositoryImpl {
    pool: Pool<Postgres>,
}

impl SessionRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

fn session_from_row(row: &PgRow) -> Session {
    let id: Uuid = row.get("id");
    let session_date: NaiveDate = row.get("session_date");
    let status: SessionStatus = row.get::<String, _>("status").into();
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: Option<DateTime<Utc>> = row.get("updated_at");

    Session::from_row(id, session_date, status, created_at, updated_at)
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
impl SessionRepository for SessionRepositoryImpl {
    async fn insert(&self, session: Session) -> HttpResult<Session> {
        let row = sqlx::query(
            r#"
            INSERT INTO volleyball.session (id, session_date, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(session.id())
        .bind(session.session_date())
        .bind(session.status().to_string())
        .bind(session.created_at())
        .bind(session.updated_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(session_from_row(&row))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Session>> {
        let row = sqlx::query(r#"SELECT * FROM volleyball.session WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| session_from_row(&r)))
    }

    async fn update(&self, session: Session) -> HttpResult<Session> {
        let row = sqlx::query(
            r#"
            UPDATE volleyball.session
            SET status = $2, updated_at = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(session.id())
        .bind(session.status().to_string())
        .bind(session.updated_at())
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("session", session.id().to_string())?;

        Ok(session_from_row(&row))
    }

    async fn list(&self, filters: &SessionFilters) -> HttpResult<Vec<Session>> {
        let mut builder = QueryBuilder::new("SELECT * FROM volleyball.session WHERE 1 = 1");

        if let Some(ids) = filters.ids() {
            builder.push(" AND id = ANY(");
            builder.push_bind(ids);
            builder.push(")");
        }

        if let Some(statuses) = filters.statuses() {
            builder.push(" AND status = ANY(");
            builder.push_bind(
                statuses
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
            builder.push(")");
        }

        if let Some(start_date) = filters.start_date() {
            builder.push(" AND session_date >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(" AND session_date <= ");
            builder.push_bind(end_date);
        }

        builder.push(" ORDER BY session_date DESC");

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows.iter().map(session_from_row).collect())
    }

    async fn add_roster_players(&self, session_id: Uuid, player_ids: &[Uuid]) -> HttpResult<()> {
        let mut tx = self.pool.begin().await?;

        for player_id in player_ids {
            sqlx::query(
                r#"
                INSERT INTO volleyball.session_player (session_id, player_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(session_id)
            .bind(player_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_roster_player_ids(&self, session_id: Uuid) -> HttpResult<Vec<Uuid>> {
        let rows =
            sqlx::query(r#"SELECT player_id FROM volleyball.session_player WHERE session_id = $1"#)
                .bind(session_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(rows.iter().map(|r| r.get("player_id")).collect())
    }

    async fn list_roster_players(&self, session_id: Uuid) -> HttpResult<Vec<Player>> {
        let rows = sqlx::query(
            r#"
            SELECT p.*
            FROM volleyball.session_player sp
            JOIN volleyball.player p ON p.id = sp.player_id
            WHERE sp.session_id = $1
            ORDER BY p.name ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(player_from_row).collect())
    }
}
