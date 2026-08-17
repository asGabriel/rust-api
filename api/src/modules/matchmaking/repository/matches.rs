use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::matchmaking::domain::matches::Match;

#[async_trait]
pub trait MatchRepository {
    async fn insert(&self, match_: Match) -> HttpResult<Match>;

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Match>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Match>>;

    async fn update(&self, match_: Match) -> HttpResult<Match>;
}

pub type DynMatchRepository = dyn MatchRepository + Send + Sync;

pub struct MatchRepositoryImpl {
    pool: Pool<Postgres>,
}

impl MatchRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl MatchRepository for MatchRepositoryImpl {
    async fn insert(&self, match_: Match) -> HttpResult<Match> {
        let row = sqlx::query(
            r#"
            INSERT INTO matchmaking.match (
                id, session_id, court, team_a_id, team_b_id, winner_team_id, started_at, played_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(*match_.id())
        .bind(*match_.session_id())
        .bind(*match_.court() as i16)
        .bind(*match_.team_a_id())
        .bind(*match_.team_b_id())
        .bind(*match_.winner_team_id())
        .bind(*match_.started_at())
        .bind(*match_.played_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Match::from(&row))
    }

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Match>> {
        let rows = sqlx::query("SELECT * FROM matchmaking.match WHERE session_id = $1")
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(Match::from).collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Match>> {
        let row = sqlx::query("SELECT * FROM matchmaking.match WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.as_ref().map(Match::from))
    }

    async fn update(&self, match_: Match) -> HttpResult<Match> {
        let row = sqlx::query(
            r#"
            UPDATE matchmaking.match SET
                winner_team_id = $2,
                played_at = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(*match_.id())
        .bind(*match_.winner_team_id())
        .bind(*match_.played_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Match::from(&row))
    }
}
