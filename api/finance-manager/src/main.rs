use std::sync::Arc;

use axum::Router;
use database::DbPool;
use finance_manager::modules::{
    payment::{handler::payment::PaymentHandlerImpl, repository::payment::PaymentRepositoryImpl},
    routes::{self, AppState},
};

#[tokio::main]
async fn main() {
    let db_conection = DbPool::new().await;
    let pool = db_conection.get_connection();

    let payment_handler = PaymentHandlerImpl {
        pool: pool.clone(),
        payment_repository: Arc::new(PaymentRepositoryImpl),
    };

    let app_state = AppState {
        db_pool: pool.clone(),
        payment_handler: Arc::new(payment_handler),
    };

    let app: Router = routes::configure_services().with_state(app_state);

    let port = std::env::var("PORT").expect("Could not fetch port data.");
    let url = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    db_conection.close().await;

}
