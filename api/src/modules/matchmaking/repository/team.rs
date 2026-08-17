use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::matchmaking::domain::team::Team;

#[async_trait]
pub trait TeamRepository {
    async fn insert(&self, team: Team) -> HttpResult<Team>;

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Team>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Team>>;
}

pub type DynTeamRepository = dyn TeamRepository + Send + Sync;

pub struct TeamRepositoryImpl {
    pool: Pool<Postgres>,
}

impl TeamRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl TeamRepository for TeamRepositoryImpl {
    /// Also used to persist an already-existing team after a mutation (see
    /// `Team::disband`/`register_win`/`add_player`) — the trait has no
    /// separate `update`, so this upserts on `id` instead.
    async fn insert(&self, team: Team) -> HttpResult<Team> {
        let status: String = (*team.status()).into();
        let player_ids = Vec::from_iter(team.player_ids().iter().copied());

        let row = sqlx::query(
            r#"
            INSERT INTO matchmaking.team (
                id, session_id, player_ids, status, consecutive_wins, priority, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                player_ids = EXCLUDED.player_ids,
                status = EXCLUDED.status,
                consecutive_wins = EXCLUDED.consecutive_wins,
                priority = EXCLUDED.priority
            RETURNING *
            "#,
        )
        .bind(*team.id())
        .bind(*team.session_id())
        .bind(player_ids)
        .bind(status)
        .bind(*team.consecutive_wins() as i16)
        .bind(team.is_priority())
        .bind(*team.created_at())
        .fetch_one(&self.pool)
        .await?;

        Ok(Team::from(&row))
    }

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Team>> {
        let rows = sqlx::query("SELECT * FROM matchmaking.team WHERE session_id = $1")
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(Team::from).collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Team>> {
        let row = sqlx::query("SELECT * FROM matchmaking.team WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.as_ref().map(Team::from))
    }
}
