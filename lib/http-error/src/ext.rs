use crate::{HttpError, HttpResult};
use std::fmt::Display;

/// Extensions to Option<T> to produce HttpError directly.
pub trait OptionHttpExt<T> {
    fn or_not_found(self, entity: impl AsRef<str>, id: impl ToString) -> HttpResult<T>;
    fn or_bad_request(self, msg: impl Into<String>) -> HttpResult<T>;
}

impl<T> OptionHttpExt<T> for Option<T> {
    fn or_not_found(self, entity: impl AsRef<str>, id: impl ToString) -> HttpResult<T> {
        self.ok_or_else(|| HttpError::not_found(entity, id))
    }

    fn or_bad_request(self, msg: impl Into<String>) -> HttpResult<T> {
        self.ok_or_else(|| HttpError::bad_request(msg.into()))
    }
}

/// Extensions to Result<T, E> to map to HttpError in an ergonomic way.
pub trait ResultHttpExt<T, E> {
    fn map_internal(self) -> HttpResult<T>
    where
        E: std::error::Error + Send + Sync + 'static;

    fn map_err_with<F>(self, f: F) -> HttpResult<T>
    where
        F: FnOnce(E) -> HttpError;

    fn conflict_if(self, predicate: bool, msg: impl Into<String>) -> HttpResult<T>
    where
        E: std::error::Error + Send + Sync + 'static;
}

impl<T, E> ResultHttpExt<T, E> for Result<T, E> {
    fn map_internal(self) -> HttpResult<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.map_err(|e| HttpError::internal("Erro interno").with_cause(e))
    }

    fn map_err_with<F>(self, f: F) -> HttpResult<T>
    where
        F: FnOnce(E) -> HttpError,
    {
        self.map_err(f)
    }

    fn conflict_if(self, predicate: bool, msg: impl Into<String>) -> HttpResult<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match self {
            Ok(v) => {
                if predicate {
                    Err(HttpError::conflict(msg.into()))
                } else {
                    Ok(v)
                }
            }
            Err(e) => Err(HttpError::internal("Erro interno").with_cause(e)),
        }
    }
}

pub fn validation_errors(pairs: impl IntoIterator<Item = (impl AsRef<str>, impl Display)>) -> HttpError {
    let map: serde_json::Map<String, serde_json::Value> = pairs
        .into_iter()
        .map(|(f, m)| (f.as_ref().to_string(), serde_json::Value::String(m.to_string())))
        .collect();

    HttpError::unprocessable(serde_json::Value::Object(map))
}
