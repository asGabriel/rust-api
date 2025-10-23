use crate::HttpError;

impl From<reqwest::Error> for HttpError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            HttpError::gateway_timeout("External request timeout")
        } else if err.is_connect() {
            HttpError::bad_gateway("Failed to connect to external service")
        } else if err.is_status() {
            let status = err.status();
            match status {
                Some(s) if s.is_client_error() => {
                    HttpError::bad_request("External request error").with_cause(err)
                }
                Some(s) if s.is_server_error() => {
                    HttpError::bad_gateway("External service returned error").with_cause(err)
                }
                _ => HttpError::internal("External request error").with_cause(err),
            }
        } else {
            HttpError::internal("External request error").with_cause(err)
        }
    }
}

impl From<reqwest::Error> for Box<HttpError> {
    fn from(err: reqwest::Error) -> Self {
        Box::new(HttpError::from(err))
    }
}
