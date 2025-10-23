use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub mod query;

#[derive(Debug, Clone)]
pub struct DbPool {
    pool: Pool<Postgres>,
}

impl DbPool {
    pub async fn new() -> Self {
        const MAX_CONNECTIONS: u32 = 5;

        // postgresql://user:password@localhost:5432/db_name
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        Self {
            pool: PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .acquire_timeout(std::time::Duration::from_secs(30))
                .idle_timeout(std::time::Duration::from_secs(600))
                .max_lifetime(std::time::Duration::from_secs(1800))
                .connect(&url)
                .await
                .expect("Failed to connect to database"),
        }
    }

    pub fn get_connection(&self) -> &Pool<Postgres> {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
