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
    auth::domain::auth_user::AuthUser,
    finance::{
        domain::income::IncomeFilters,
        handler::income::use_cases::{CreateIncomeRequest, UpdateIncomeRequest},
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    let main_income_routes = Router::new()
        .route("/list", post(list_incomes))
        .route("/", post(create_income));

    let income_id_routes = Router::new().nest(
        "/{income_id}",
        Router::new().route("/", patch(update_income).delete(soft_delete_income)),
    );

    Router::new().nest(
        "/income",
        Router::new()
            .merge(main_income_routes)
            .merge(income_id_routes),
    )
}

async fn create_income(
    state: State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<CreateIncomeRequest>,
) -> HttpResult<impl IntoResponse> {
    let income = state
        .finance_state
        .income_handler
        .register_new_income(auth_user.tenant_id(), request)
        .await?;

    Ok(Json(income))
}

async fn list_incomes(
    state: State<AppState>,
    auth_user: AuthUser,
    Json(filters): Json<IncomeFilters>,
) -> HttpResult<impl IntoResponse> {
    let incomes = state
        .finance_state
        .income_handler
        .list_incomes(auth_user.tenant_id(), &filters)
        .await?;

    Ok(Json(incomes))
}

async fn update_income(
    state: State<AppState>,
    auth_user: AuthUser,
    Path(income_id): Path<Uuid>,
    Json(request): Json<UpdateIncomeRequest>,
) -> HttpResult<impl IntoResponse> {
    let income = state
        .finance_state
        .income_handler
        .update_income(auth_user.tenant_id(), income_id, request)
        .await?;

    Ok(Json(income))
}

async fn soft_delete_income(
    state: State<AppState>,
    auth_user: AuthUser,
    Path(income_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    state
        .finance_state
        .income_handler
        .soft_delete_income(auth_user.tenant_id(), auth_user.user_id(), income_id)
        .await?;

    Ok(StatusCode::OK)
}
