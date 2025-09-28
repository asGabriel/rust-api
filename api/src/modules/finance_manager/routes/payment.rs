use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use http_error::HttpResult;

use crate::modules::routes::AppState;

pub fn configure_routes() -> Router<AppState> {
    Router::new().route("/payment", get(get_payment))
}

async fn get_payment(_state: State<AppState>) -> HttpResult<impl IntoResponse> {
    Ok(Json("OK"))
}
