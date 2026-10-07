use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    auth::domain::auth_user::AuthUser,
    finance::handler::list::use_cases::{CreateListRequest, UpdateListRequest},
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    let main_list_routes = Router::new().route("/", get(list_lists).post(create_list));

    let list_id_routes = Router::new().nest(
        "/{list_id}",
        Router::new().route("/", patch(update_list).delete(delete_list)),
    );

    Router::new().nest(
        "/list",
        Router::new().merge(main_list_routes).merge(list_id_routes),
    )
}

async fn create_list(
    state: State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<CreateListRequest>,
) -> HttpResult<impl IntoResponse> {
    let list = state
        .finance_state
        .list_handler
        .register_new_list(auth_user.tenant_id(), request)
        .await?;

    Ok(Json(list))
}

async fn list_lists(state: State<AppState>, auth_user: AuthUser) -> HttpResult<impl IntoResponse> {
    let lists = state
        .finance_state
        .list_handler
        .list_lists(auth_user.tenant_id())
        .await?;

    Ok(Json(lists))
}

async fn update_list(
    state: State<AppState>,
    auth_user: AuthUser,
    Path(list_id): Path<Uuid>,
    Json(request): Json<UpdateListRequest>,
) -> HttpResult<impl IntoResponse> {
    let list = state
        .finance_state
        .list_handler
        .update_list(auth_user.tenant_id(), list_id, request)
        .await?;
    Ok(Json(list))
}

async fn delete_list(
    state: State<AppState>,
    auth_user: AuthUser,
    Path(list_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    state
        .finance_state
        .list_handler
        .delete_list(auth_user.tenant_id(), list_id)
        .await?;

    Ok(StatusCode::OK)
}
