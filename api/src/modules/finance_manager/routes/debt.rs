use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::{
        domain::debt::DebtFilters,
        handler::debt::use_cases::{CreateCategoryRequest, CreateDebtRequest},
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    let category_routes = Router::new().nest(
        "/category",
        Router::new()
            .route("/list", post(list_categories))
            .route("/", post(create_category)),
    );

    Router::new().nest(
        "/debt",
        Router::new()
            .route("/list", post(list_debts))
            .route("/", post(create_debt))
            .merge(category_routes),
    )
}

async fn create_category(
    state: State<AppState>,
    Json(request): Json<CreateCategoryRequest>,
) -> HttpResult<impl IntoResponse> {
    let category = state
        .finance_manager_state
        .debt_handler
        .create_debt_category(request)
        .await?;

    Ok(Json(category))
}

async fn list_categories(state: State<AppState>) -> HttpResult<impl IntoResponse> {
    let categories = state
        .finance_manager_state
        .debt_handler
        .list_debt_categories()
        .await?;

    Ok(Json(categories))
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
