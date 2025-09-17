#[cfg(feature = "sqlx")]
use sqlx::{error::DatabaseError, Error};

use crate::HttpError;

#[cfg(feature = "sqlx")]
impl From<Error> for HttpError {
    fn from(err: Error) -> Self {
        match err {
            Error::RowNotFound => {
                // Deixe a camada de aplicação decidir a entidade/id quando possível.
                // Aqui retornamos um 404 genérico.
                HttpError::not_found("Recurso", "não encontrado")
            }
            Error::Database(db_err) => map_database_error(db_err.as_ref()),
            _ => HttpError::internal("Erro de banco de dados").with_cause(err),
        }
    }
}

#[cfg(feature = "sqlx")]
fn map_database_error(db_err: &(dyn DatabaseError + 'static)) -> HttpError {
    // Para Postgres, códigos de erro seguem SQLSTATE.
    // 23505 = unique_violation, 23503 = foreign_key_violation
    let code = db_err.code();

    if let Some(code) = code.as_deref() {
        match code {
            "23505" => {
                HttpError::conflict("Violação de unicidade").with_meta(meta_code(code))
            }
            "23503" => {
                HttpError::bad_request("Violação de chave estrangeira").with_meta(meta_code(code))
            }
            _ => HttpError::internal("Erro de banco de dados").with_meta(meta_code(code)),
        }
    } else {
        HttpError::internal("Erro de banco de dados")
    }
}

#[cfg(feature = "sqlx")]
fn meta_code(code: &str) -> serde_json::Value {
    serde_json::json!({ "sqlstate": code })
}
