use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::matchmaking::domain::session::Session;

#[async_trait]
pub trait SessionRepository {
    async fn insert(&self, session: Session) -> HttpResult<Session>;

    async fn list(&self) -> HttpResult<Vec<Session>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Session>>;

    async fn update(&self, session: Session) -> HttpResult<Session>;
}

pub type DynSessionRepository = dyn SessionRepository + Send + Sync;

pub struct SessionRepositoryImpl {
    pool: Pool<Postgres>,
}

impl SessionRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl SessionRepository for SessionRepositoryImpl {
    async fn insert(&self, session: Session) -> HttpResult<Session> {
        let game_mode: String = (*session.game_mode()).into();
        let shuffle_type: String = (*session.shuffle_type()).into();
        let player_ids = Vec::from_iter(session.player_ids().iter().copied());

        let row = sqlx::query(
            r#"
            INSERT INTO matchmaking.session (
                id, date, players_per_team, sets_to_win, points_per_set,
                available_courts, game_mode, shuffle_type, player_ids,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(*session.id())
        .bind(*session.date())
        .bind(*session.settings().players_per_team() as i16)
        .bind(*session.settings().sets_to_win() as i16)
        .bind(*session.settings().points_per_set() as i16)
        .bind(*session.available_courts() as i16)
        .bind(game_mode)
        .bind(shuffle_type)
        .bind(player_ids)
        .bind(*session.created_at())
        .bind(*session.updated_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Session::from(&row))
    }

    async fn list(&self) -> HttpResult<Vec<Session>> {
        let rows = sqlx::query("SELECT * FROM matchmaking.session")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(Session::from).collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Session>> {
        let row = sqlx::query("SELECT * FROM matchmaking.session WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.as_ref().map(Session::from))
    }

    async fn update(&self, session: Session) -> HttpResult<Session> {
        let game_mode: String = (*session.game_mode()).into();
        let shuffle_type: String = (*session.shuffle_type()).into();
        let player_ids = Vec::from_iter(session.player_ids().iter().copied());

        let row = sqlx::query(
            r#"
            UPDATE matchmaking.session SET
                date = $2,
                players_per_team = $3,
                sets_to_win = $4,
                points_per_set = $5,
                available_courts = $6,
                game_mode = $7,
                shuffle_type = $8,
                player_ids = $9,
                updated_at = $10
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(*session.id())
        .bind(*session.date())
        .bind(*session.settings().players_per_team() as i16)
        .bind(*session.settings().sets_to_win() as i16)
        .bind(*session.settings().points_per_set() as i16)
        .bind(*session.available_courts() as i16)
        .bind(game_mode)
        .bind(shuffle_type)
        .bind(player_ids)
        .bind(*session.updated_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Session::from(&row))
    }
}
