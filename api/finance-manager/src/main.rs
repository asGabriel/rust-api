// use finance_manager::modules::payment::*;

use database::PgPool;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    println!("Hello, world!");

    let db_conection = PgPool::new().await;

    let pool = db_conection.get_connection();

    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(pool)
        .await?;

    println!("row: {:?}", row);

    Ok(())
}
