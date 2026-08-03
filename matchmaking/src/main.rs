use axum::Router;
use database::DbPool;
use matchmaking::modules::routes::{self, AppState};

#[tokio::main]
async fn main() {
    let db_conection = DbPool::new().await;
    let pool = db_conection.get_connection();

    let app_state = AppState {
        db_pool: pool.clone(),
    };

    let app: Router = routes::configure_services().with_state(app_state);

    let port = std::env::var("MATCHMAKING_PORT").unwrap_or_else(|_| "8081".to_string());
    let url = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    db_conection.close().await;
}
