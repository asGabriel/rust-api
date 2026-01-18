use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::{
        handler::income::use_cases::CreateIncomeRequest,
        repository::income::use_cases::IncomeListFilters,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/income",
        Router::new()
            .route("/", post(create_income))
            .route("/list", post(list_incomes)),
    )
}

async fn create_income(
    state: State<AppState>,
    Json(request): Json<CreateIncomeRequest>,
) -> HttpResult<impl IntoResponse> {
    let income = state
        .finance_manager_state
        .income_handler
        .create_income(request)
        .await?;

    Ok(Json(income))
}

async fn list_incomes(
    state: State<AppState>,
    Json(filters): Json<IncomeListFilters>,
) -> HttpResult<impl IntoResponse> {
    let incomes = state
        .finance_manager_state
        .income_handler
        .list_incomes(filters)
        .await?;

    Ok(Json(incomes))
}
