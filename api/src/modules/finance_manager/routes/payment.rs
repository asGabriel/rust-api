use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::handler::payment::use_cases::CreatePaymentRequest, routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest("/payment", Router::new().route("/", post(create_payment)))
}

async fn create_payment(
    state: State<AppState>,
    Json(request): Json<CreatePaymentRequest>,
) -> HttpResult<impl IntoResponse> {
    let payment = state
        .finance_manager_state
        .payment_handler
        .create_payment(request)
        .await?;

    Ok(Json(payment))
}
