use std::sync::Arc;

use axum::Router;

use crate::modules::finance_manager::{self, FinanceManagerState};

#[derive(Clone)]
pub struct AppState {
    pub finance_manager_state: Arc<FinanceManagerState>,
}

pub fn configure_services() -> Router<AppState> {
    Router::new().merge(finance_manager::routes::payment::configure_routes())
}
