use axum::{
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::handler::financial_instrument::use_cases::{
        CreateFinancialInstrumentRequest, FinancialInstrumentListFilters,
        UpdateFinancialInstrumentRequest,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/financialInstrument",
        Router::new()
            .route("/", post(create_financial_instrument))
            .route("/list", post(list_financial_instruments))
            .route("/", patch(update_financial_instrument)),
    )
}

async fn update_financial_instrument(
    state: State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateFinancialInstrumentRequest>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let instrument = state
        .finance_manager_state
        .financial_instrument_handler
        .update_financial_instrument(*user.client_id(), request)
        .await?;

    Ok(Json(instrument))
}

async fn create_financial_instrument(
    state: State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateFinancialInstrumentRequest>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let instrument = state
        .finance_manager_state
        .financial_instrument_handler
        .create_financial_instrument(*user.client_id(), request)
        .await?;

    Ok(Json(instrument))
}

async fn list_financial_instruments(
    state: State<AppState>,
    headers: HeaderMap,
    Json(filters): Json<FinancialInstrumentListFilters>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let instruments = state
        .finance_manager_state
        .financial_instrument_handler
        .list_financial_instruments(*user.client_id(), filters)
        .await?;

    Ok(Json(instruments))
}
