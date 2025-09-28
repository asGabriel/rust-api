#[cfg(feature = "axum")]
use axum::{
    Json,
    response::{IntoResponse, Response},
};

#[cfg(feature = "http")]
use http::{HeaderValue, header};

use crate::HttpError;

#[cfg(feature = "axum")]
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        #[cfg(feature = "http")]
        let status = self.status();

        #[cfg(not(feature = "http"))]
        let status = http::StatusCode::from_u16(self.status_u16())
            .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);

        let body = self.to_problem_details();

        let mut res = (status, Json(body)).into_response();

        #[cfg(feature = "http")]
        if let Some(tid) = self.trace_id.as_deref() {
            if let Ok(val) = HeaderValue::from_str(tid) {
                res.headers_mut()
                    .insert(header::HeaderName::from_static("x-trace-id"), val);
            }
        }

        res
    }
}
