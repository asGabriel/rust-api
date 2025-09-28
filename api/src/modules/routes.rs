use std::sync::Arc;

use axum::Router;
use sqlx::{Pool, Postgres};

use crate::modules::finance_manager::{self, handler::payment::DynPaymentHandler};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<Postgres>,
    pub payment_handler: Arc<DynPaymentHandler>,
}

pub fn configure_services() -> Router<AppState> {
    Router::new().merge(finance_manager::routes::payment::configure_routes())
}
