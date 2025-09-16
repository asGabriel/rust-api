use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

#[derive(Debug, Clone)]
pub struct PgPool {
    pool: Pool<Postgres>,
}

impl PgPool {
    pub async fn new() -> Self {
        const MAX_CONNECTIONS: u32 = 5;

        // postgresql://user:password@localhost:5432/db_name
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        Self {
            pool: PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(&url)
                .await
                .expect("Failed to connect to database"),
        }
    }

    pub fn get_connection(&self) -> &Pool<Postgres> {
        &self.pool
    }
}
