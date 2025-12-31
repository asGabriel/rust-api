use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::{
        domain::debt::{installment::InstallmentFilters, DebtFilters},
        handler::debt::use_cases::CreateDebtRequest,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    let main_debt_routes = Router::new()
        .route("/list", post(list_debts))
        .route("/", post(create_debt));

    let installment_routes = Router::new().nest(
        "/installment",
        Router::new().route("/list", post(list_debt_installments)),
    );

    Router::new().nest(
        "/debt",
        Router::new()
            .merge(main_debt_routes)
            .merge(installment_routes),
    )
}

async fn list_debt_installments(
    state: State<AppState>,
    Json(filters): Json<InstallmentFilters>,
) -> HttpResult<impl IntoResponse> {
    let installments = state
        .finance_manager_state
        .debt_handler
        .list_debt_installments(&filters)
        .await?;

    Ok(Json(installments))
}

async fn create_debt(
    state: State<AppState>,
    Json(request): Json<CreateDebtRequest>,
) -> HttpResult<impl IntoResponse> {
    let debt = state
        .finance_manager_state
        .debt_handler
        .register_new_debt(request)
        .await?;

    Ok(Json(debt))
}

pub async fn list_debts(
    state: State<AppState>,
    Json(filters): Json<DebtFilters>,
) -> HttpResult<impl IntoResponse> {
    let debts = state
        .finance_manager_state
        .debt_handler
        .list_debts(&filters)
        .await?;

    Ok(Json(debts))
}
