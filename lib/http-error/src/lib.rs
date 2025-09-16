pub mod error;
pub mod ext;
pub mod problem;

#[cfg(feature = "axum")]
pub mod axum_integration;

#[cfg(feature = "sqlx")]
pub mod sqlx_integration;

pub use error::{HttpError, HttpErrorKind, HttpResult};
