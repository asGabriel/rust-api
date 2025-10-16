use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::{domain::debt::DebtFilters, handler::debt::CreateDebtRequest},
    worker::WorkerTopic,
    AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/debt",
        Router::new()
            .route("/list", post(list_debts))
            .route("/", post(create_debt)),
    )
}

async fn create_debt(
    state: State<AppState>,
    Json(request): Json<CreateDebtRequest>,
) -> HttpResult<impl IntoResponse> {
    let debt = state
        .finance_manager_state
        .debt_handler
        .create_debt(request)
        .await?;

    state.worker_state.notify(
        WorkerTopic::DebtCreated,
        "Nova dívida criada".to_string(),
        serde_json::to_value(&debt).ok(),
    );

    Ok(Json(debt))
}

pub async fn list_debts(
    state: State<AppState>,
    Json(filters): Json<DebtFilters>,
) -> HttpResult<impl IntoResponse> {
    let debts = state
        .finance_manager_state
        .debt_handler
        .list_debts(filters)
        .await?;

    Ok(Json(debts))
}
