use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::matchmaking::domain::queue::QueueEntry;

#[async_trait]
pub trait SessionQueueRepository {
    /// Every waiting player in the session, unordered — the caller sorts
    /// with `domain::queue::order_queue`.
    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<QueueEntry>>;

    /// Adds `entries` to the queue in one statement. Used when a session is
    /// created (all confirmed players), when a player checks in, and when
    /// players come back off a court. New entries are never pinned. A player
    /// who already has a queue row in the session is left as-is — the insert
    /// is a no-op for them, so a redundant check-in or a stale row can never
    /// raise a `UNIQUE (session_id, player_id)` error.
    async fn insert_many(&self, entries: &[QueueEntry]) -> HttpResult<()>;

    /// Removes `player_ids` from the session's queue in one statement (they
    /// are entering a court), returning the rows that were actually removed
    /// so the caller keeps each player's `games_played`. Atomic: two
    /// concurrent removes can never both take the same player.
    async fn remove_players(
        &self,
        session_id: &Uuid,
        player_ids: &[Uuid],
    ) -> HttpResult<Vec<QueueEntry>>;

    async fn set_pinned(
        &self,
        session_id: &Uuid,
        player_id: &Uuid,
        pinned: bool,
    ) -> HttpResult<Option<QueueEntry>>;
}

pub type DynSessionQueueRepository = dyn SessionQueueRepository + Send + Sync;

pub struct SessionQueueRepositoryImpl {
    pool: Pool<Postgres>,
}

impl SessionQueueRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl SessionQueueRepository for SessionQueueRepositoryImpl {
    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<QueueEntry>> {
        let rows = sqlx::query("SELECT * FROM matchmaking.session_queue WHERE session_id = $1")
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(QueueEntry::from).collect())
    }

    async fn insert_many(&self, entries: &[QueueEntry]) -> HttpResult<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let ids: Vec<Uuid> = entries.iter().map(|e| *e.id()).collect();
        let session_ids: Vec<Uuid> = entries.iter().map(|e| *e.session_id()).collect();
        let player_ids: Vec<Uuid> = entries.iter().map(|e| *e.player_id()).collect();
        let games_played: Vec<i16> = entries.iter().map(|e| *e.games_played() as i16).collect();
        let enqueued_at: Vec<_> = entries.iter().map(|e| *e.enqueued_at()).collect();

        sqlx::query(
            r#"
            INSERT INTO matchmaking.session_queue
                (id, session_id, player_id, games_played, enqueued_at)
            SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::smallint[], $5::timestamptz[])
            ON CONFLICT (session_id, player_id) DO NOTHING
            "#,
        )
        .bind(&ids)
        .bind(&session_ids)
        .bind(&player_ids)
        .bind(&games_played)
        .bind(&enqueued_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_players(
        &self,
        session_id: &Uuid,
        player_ids: &[Uuid],
    ) -> HttpResult<Vec<QueueEntry>> {
        if player_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"
            DELETE FROM matchmaking.session_queue
            WHERE session_id = $1 AND player_id = ANY($2)
            RETURNING *
            "#,
        )
        .bind(session_id)
        .bind(player_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(QueueEntry::from).collect())
    }

    async fn set_pinned(
        &self,
        session_id: &Uuid,
        player_id: &Uuid,
        pinned: bool,
    ) -> HttpResult<Option<QueueEntry>> {
        let row = sqlx::query(
            r#"
            UPDATE matchmaking.session_queue
            SET pinned = $3,
                pinned_at = CASE WHEN $3 THEN now() ELSE NULL END
            WHERE session_id = $1 AND player_id = $2
            RETURNING *
            "#,
        )
        .bind(session_id)
        .bind(player_id)
        .bind(pinned)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.as_ref().map(QueueEntry::from))
    }
}
