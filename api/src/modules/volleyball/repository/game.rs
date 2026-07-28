use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};
use sqlx::{Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::modules::volleyball::domain::{
    draw::PlayerStanding,
    game::{Game, GameFilters},
};

#[async_trait]
pub trait GameRepository {
    async fn insert(&self, game: Game) -> HttpResult<Game>;

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Game>>;

    async fn update(&self, game: Game) -> HttpResult<Game>;

    async fn list(&self, filters: &GameFilters) -> HttpResult<Vec<Game>>;

    /// Most recent *other* finished game on the same court (used to check the
    /// "2 consecutive wins" rule). `None` if this was the court's first
    /// finished game.
    async fn get_previous_finished_on_court(
        &self,
        session_id: Uuid,
        court: i16,
        exclude_game_id: Uuid,
    ) -> HttpResult<Option<Game>>;

    /// Most recently finished game across the whole session (any court) —
    /// used to derive the current cooldown set for display purposes.
    async fn get_most_recently_finished_game(&self, session_id: Uuid) -> HttpResult<Option<Game>>;

    async fn list_players_in_pending_games(&self, session_id: Uuid) -> HttpResult<Vec<Uuid>>;

    /// Aggregates games played / wins for every roster player that isn't
    /// currently busy (in a pending game) or explicitly excluded (e.g. a
    /// retained pair), as raw input for `domain::draw::draw`.
    async fn compute_eligible_standings(
        &self,
        session_id: Uuid,
        excluded_player_ids: &[Uuid],
    ) -> HttpResult<Vec<PlayerStanding>>;
}

pub type DynGameRepository = dyn GameRepository + Send + Sync;

#[derive(Clone)]
pub struct GameRepositoryImpl {
    pool: Pool<Postgres>,
}

impl GameRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn insert(&self, game: Game) -> HttpResult<Game> {
        let dto = entity::GameEntity::from(&game);

        let row = sqlx::query(
            r#"
            INSERT INTO volleyball.game (
                id, session_id, court,
                team_a_player1_id, team_a_player2_id,
                team_b_player1_id, team_b_player2_id,
                winner, created_at, finished_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(dto.id)
        .bind(dto.session_id)
        .bind(dto.court)
        .bind(dto.team_a_player1_id)
        .bind(dto.team_a_player2_id)
        .bind(dto.team_b_player1_id)
        .bind(dto.team_b_player2_id)
        .bind(&dto.winner)
        .bind(dto.created_at)
        .bind(dto.finished_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(entity::GameEntity::from(&row).into())
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Game>> {
        let row = sqlx::query(r#"SELECT * FROM volleyball.game WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| entity::GameEntity::from(&r).into()))
    }

    async fn update(&self, game: Game) -> HttpResult<Game> {
        let dto = entity::GameEntity::from(&game);

        let row = sqlx::query(
            r#"
            UPDATE volleyball.game
            SET winner = $2, finished_at = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(dto.id)
        .bind(&dto.winner)
        .bind(dto.finished_at)
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("game", dto.id.to_string())?;

        Ok(entity::GameEntity::from(&row).into())
    }

    async fn list(&self, filters: &GameFilters) -> HttpResult<Vec<Game>> {
        let mut builder = QueryBuilder::new("SELECT * FROM volleyball.game WHERE 1 = 1");

        if let Some(session_id) = filters.session_id() {
            builder.push(" AND session_id = ");
            builder.push_bind(*session_id);
        }

        if let Some(court) = filters.court() {
            builder.push(" AND court = ");
            builder.push_bind(*court);
        }

        if let Some(true) = filters.pending_only() {
            builder.push(" AND winner IS NULL");
        }

        builder.push(" ORDER BY created_at ASC");

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows
            .iter()
            .map(|r| entity::GameEntity::from(r).into())
            .collect())
    }

    async fn get_previous_finished_on_court(
        &self,
        session_id: Uuid,
        court: i16,
        exclude_game_id: Uuid,
    ) -> HttpResult<Option<Game>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM volleyball.game
            WHERE session_id = $1 AND court = $2 AND winner IS NOT NULL AND id != $3
            ORDER BY finished_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .bind(court)
        .bind(exclude_game_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| entity::GameEntity::from(&r).into()))
    }

    async fn get_most_recently_finished_game(&self, session_id: Uuid) -> HttpResult<Option<Game>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM volleyball.game
            WHERE session_id = $1 AND winner IS NOT NULL
            ORDER BY finished_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| entity::GameEntity::from(&r).into()))
    }

    async fn list_players_in_pending_games(&self, session_id: Uuid) -> HttpResult<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT unnest(ARRAY[
                team_a_player1_id, team_a_player2_id,
                team_b_player1_id, team_b_player2_id
            ]) AS player_id
            FROM volleyball.game
            WHERE session_id = $1 AND winner IS NULL
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("player_id")).collect())
    }

    async fn compute_eligible_standings(
        &self,
        session_id: Uuid,
        excluded_player_ids: &[Uuid],
    ) -> HttpResult<Vec<PlayerStanding>> {
        let rows = sqlx::query(
            r#"
            WITH roster AS (
                SELECT player_id FROM volleyball.session_player WHERE session_id = $1
            ),
            busy AS (
                SELECT DISTINCT unnest(ARRAY[
                    team_a_player1_id, team_a_player2_id,
                    team_b_player1_id, team_b_player2_id
                ]) AS player_id
                FROM volleyball.game
                WHERE session_id = $1 AND winner IS NULL
            ),
            finished_appearances AS (
                SELECT team_a_player1_id AS player_id, winner = 'TEAM_A' AS won
                FROM volleyball.game WHERE session_id = $1 AND winner IS NOT NULL
                UNION ALL
                SELECT team_a_player2_id, winner = 'TEAM_A'
                FROM volleyball.game WHERE session_id = $1 AND winner IS NOT NULL
                UNION ALL
                SELECT team_b_player1_id, winner = 'TEAM_B'
                FROM volleyball.game WHERE session_id = $1 AND winner IS NOT NULL
                UNION ALL
                SELECT team_b_player2_id, winner = 'TEAM_B'
                FROM volleyball.game WHERE session_id = $1 AND winner IS NOT NULL
            )
            SELECT
                r.player_id AS player_id,
                COUNT(f.player_id)::int AS games_played,
                COALESCE(SUM(CASE WHEN f.won THEN 1 ELSE 0 END), 0)::int AS wins
            FROM roster r
            LEFT JOIN finished_appearances f ON f.player_id = r.player_id
            WHERE r.player_id NOT IN (SELECT player_id FROM busy)
              AND r.player_id != ALL($2)
            GROUP BY r.player_id
            "#,
        )
        .bind(session_id)
        .bind(excluded_player_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| PlayerStanding {
                player_id: r.get("player_id"),
                games_played: r.get("games_played"),
                wins: r.get("wins"),
            })
            .collect())
    }
}

pub mod entity {
    use chrono::{DateTime, Utc};
    use sqlx::postgres::PgRow;
    use sqlx::Row;
    use uuid::Uuid;

    use crate::modules::volleyball::domain::game::{Game, GameWinner, Pair};

    pub struct GameEntity {
        pub id: Uuid,
        pub session_id: Uuid,
        pub court: i16,
        pub team_a_player1_id: Uuid,
        pub team_a_player2_id: Uuid,
        pub team_b_player1_id: Uuid,
        pub team_b_player2_id: Uuid,
        pub winner: Option<String>,
        pub created_at: DateTime<Utc>,
        pub finished_at: Option<DateTime<Utc>>,
    }

    impl From<&PgRow> for GameEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                session_id: row.get("session_id"),
                court: row.get("court"),
                team_a_player1_id: row.get("team_a_player1_id"),
                team_a_player2_id: row.get("team_a_player2_id"),
                team_b_player1_id: row.get("team_b_player1_id"),
                team_b_player2_id: row.get("team_b_player2_id"),
                winner: row.get("winner"),
                created_at: row.get("created_at"),
                finished_at: row.get("finished_at"),
            }
        }
    }

    impl From<&Game> for GameEntity {
        fn from(game: &Game) -> Self {
            let [a1, a2] = game.team_a().players();
            let [b1, b2] = game.team_b().players();

            Self {
                id: *game.id(),
                session_id: *game.session_id(),
                court: *game.court(),
                team_a_player1_id: a1,
                team_a_player2_id: a2,
                team_b_player1_id: b1,
                team_b_player2_id: b2,
                winner: game.winner().map(|w| w.to_string()),
                created_at: *game.created_at(),
                finished_at: *game.finished_at(),
            }
        }
    }

    impl From<GameEntity> for Game {
        fn from(dto: GameEntity) -> Self {
            Game::from_row(
                dto.id,
                dto.session_id,
                dto.court,
                Pair::new(dto.team_a_player1_id, dto.team_a_player2_id),
                Pair::new(dto.team_b_player1_id, dto.team_b_player2_id),
                dto.winner.map(GameWinner::from),
                dto.created_at,
                dto.finished_at,
            )
        }
    }
}
