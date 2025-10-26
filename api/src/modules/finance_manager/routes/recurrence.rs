use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::handler::recurrence::use_cases::CreateRecurrenceRequest, routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/recurrence",
        Router::new()
            .route("/", post(create_recurrence))
            .route("/list", post(list_recurrences)),
    )
}

async fn create_recurrence(
    state: State<AppState>,
    Json(request): Json<CreateRecurrenceRequest>,
) -> HttpResult<impl IntoResponse> {
    let recurrence = state
        .finance_manager_state
        .recurrence_handler
        .create_recurrence(request)
        .await?;

    Ok(Json(recurrence))
}

async fn list_recurrences(state: State<AppState>) -> HttpResult<impl IntoResponse> {
    let recurrences = state
        .finance_manager_state
        .recurrence_handler
        .list_recurrences()
        .await?;

    Ok(Json(recurrences))
}
