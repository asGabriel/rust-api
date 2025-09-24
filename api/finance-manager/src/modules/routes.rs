use std::sync::Arc;

use axum::Router;
use sqlx::{Pool, Postgres};

use crate::modules::payment::handler::payment::DynPaymentHandler;

pub mod payment;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<Postgres>,
    pub payment_handler: Arc<DynPaymentHandler>,
}

pub fn configure_services() -> Router<AppState> {
    Router::new().merge(payment::configure_routes())
}
