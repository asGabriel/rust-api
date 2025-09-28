use std::{borrow::Cow, error::Error as StdError, fmt};

#[cfg(feature = "http")]
use http::StatusCode;

use crate::problem::ProblemDetails;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpErrorKind {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
    TooManyRequests,
    Internal,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
}

impl HttpErrorKind {
    #[cfg(feature = "http")]
    pub fn status(self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadGateway => StatusCode::BAD_GATEWAY,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::Conflict => "Conflict",
            Self::UnprocessableEntity => "Unprocessable Entity",
            Self::TooManyRequests => "Too Many Requests",
            Self::Internal => "Internal Server Error",
            Self::BadGateway => "Bad Gateway",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::GatewayTimeout => "Gateway Timeout",
        }
    }
}

/// Erro principal da lib.
#[derive(Debug)]
pub struct HttpError {
    pub kind: HttpErrorKind,
    pub message: Cow<'static, str>,
    /// Opcional: detalhes serializáveis (ex.: erros de validação),
    /// vão para `ProblemDetails.errors`.
    pub details: Option<serde_json::Value>,
    /// Opcional: causa para logs (não serializada no corpo).
    cause: Option<Box<dyn StdError + Send + Sync>>,
    /// Opcional: “instance” RFC 7807 (ex.: path do request).
    pub instance: Option<String>,
    /// Opcional: “type” RFC 7807 (URI)
    pub problem_type: Option<String>,
    /// Opcional: trace id para correlação
    pub trace_id: Option<String>,
    /// Opcional: metadados adicionais
    pub meta: Option<serde_json::Value>,
}

pub type HttpResult<T> = Result<T, HttpError>;

impl HttpError {
    fn new(kind: HttpErrorKind, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
            cause: None,
            instance: None,
            problem_type: None,
            trace_id: None,
            meta: None,
        }
    }

    // Constructors
    pub fn bad_request(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::new(HttpErrorKind::BadRequest, msg)
    }
    pub fn unauthorized(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::new(HttpErrorKind::Unauthorized, msg)
    }
    pub fn forbidden(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::new(HttpErrorKind::Forbidden, msg)
    }
    pub fn not_found(entity: impl AsRef<str>, id: impl ToString) -> Self {
        let msg = format!("{} {} não encontrado", entity.as_ref(), id.to_string());
        Self::new(HttpErrorKind::NotFound, msg)
    }
    pub fn conflict(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::new(HttpErrorKind::Conflict, msg)
    }
    pub fn unprocessable(details: serde_json::Value) -> Self {
        let mut e = Self::new(HttpErrorKind::UnprocessableEntity, "Dados inválidos");
        e.details = Some(details);
        e
    }
    pub fn too_many_requests(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::new(HttpErrorKind::TooManyRequests, msg)
    }
    pub fn internal(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::new(HttpErrorKind::Internal, msg)
    }

    // Builders
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    pub fn with_cause(mut self, cause: impl StdError + Send + Sync + 'static) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }
    pub fn with_type(mut self, problem_type: impl Into<String>) -> Self {
        self.problem_type = Some(problem_type.into());
        self
    }
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
    pub fn with_meta(mut self, meta: serde_json::Value) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn to_problem_details(&self) -> ProblemDetails {
        ProblemDetails {
            r#type: self
                .problem_type
                .clone()
                .unwrap_or_else(|| "about:blank".to_string()),
            title: self.kind.title().to_string(),
            status: self.status_u16(),
            detail: Some(self.message.to_string()),
            instance: self.instance.clone(),
            trace_id: self.trace_id.clone(),
            errors: self.details.clone(),
            meta: self.meta.clone(),
        }
    }

    #[cfg(feature = "http")]
    pub fn status(&self) -> http::StatusCode {
        self.kind.status()
    }

    pub fn status_u16(&self) -> u16 {
        #[cfg(feature = "http")]
        {
            return self.kind.status().as_u16();
        }
        #[allow(unreachable_code)]
        match self.kind {
            HttpErrorKind::BadRequest => 400,
            HttpErrorKind::Unauthorized => 401,
            HttpErrorKind::Forbidden => 403,
            HttpErrorKind::NotFound => 404,
            HttpErrorKind::Conflict => 409,
            HttpErrorKind::UnprocessableEntity => 422,
            HttpErrorKind::TooManyRequests => 429,
            HttpErrorKind::Internal => 500,
            HttpErrorKind::BadGateway => 502,
            HttpErrorKind::ServiceUnavailable => 503,
            HttpErrorKind::GatewayTimeout => 504,
        }
    }

    /// Access to the cause for logging.
    pub fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause.as_deref().map(|e| e as _)
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.kind.title(),
            self.status_u16(),
            self.message
        )
    }
}

impl StdError for HttpError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source()
    }
}
