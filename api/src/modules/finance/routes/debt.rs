use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    finance::{
        domain::debt::DebtFilters,
        handler::debt::use_cases::{CreateDebtRequest, UpdateDebtRequest},
    },
    routes::AppState,
};

// Routes are unauthenticated for now, so every request acts as this single client/user.
const ANONYMOUS_ID: Uuid = Uuid::nil();

pub fn configure_routes() -> Router<AppState> {
    let main_debt_routes = Router::new()
        .route("/list", post(list_debts))
        .route("/", post(create_debt));

    let debt_id_routes = Router::new().nest(
        "/{debt_id}",
        Router::new().route("/", patch(update_debt).delete(soft_delete_debt)),
    );

    Router::new().nest(
        "/debt",
        Router::new().merge(main_debt_routes).merge(debt_id_routes),
    )
}

async fn create_debt(
    state: State<AppState>,
    Json(request): Json<CreateDebtRequest>,
) -> HttpResult<impl IntoResponse> {
    let debt = state
        .finance_state
        .debt_handler
        .register_new_debt(ANONYMOUS_ID, request)
        .await?;

    Ok(Json(debt))
}

async fn list_debts(
    state: State<AppState>,
    Json(filters): Json<DebtFilters>,
) -> HttpResult<impl IntoResponse> {
    let debts = state
        .finance_state
        .debt_handler
        .list_debts(ANONYMOUS_ID, &filters)
        .await?;

    Ok(Json(debts))
}

async fn update_debt(
    state: State<AppState>,
    Path(debt_id): Path<Uuid>,
    Json(request): Json<UpdateDebtRequest>,
) -> HttpResult<impl IntoResponse> {
    let debt = state
        .finance_state
        .debt_handler
        .update_debt(ANONYMOUS_ID, debt_id, request)
        .await?;
    Ok(Json(debt))
}

async fn soft_delete_debt(
    state: State<AppState>,
    Path(debt_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    state
        .finance_state
        .debt_handler
        .soft_delete_debt(ANONYMOUS_ID, ANONYMOUS_ID, debt_id)
        .await?;

    Ok(StatusCode::OK)
}
