use serde::{Deserialize, Serialize};

/// RFC 7807 - Problem Details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// URI identificando o tipo de problema (ou "about:blank")
    #[serde(default = "default_type")]
    pub r#type: String,
    /// Título curto entendível por humanos
    pub title: String,
    /// HTTP status code
    pub status: u16,
    /// Detalhe específico deste caso
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// URI do recurso/endpoint que causou o problema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Id de rastreamento (para correlação em logs/observabilidade)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Campo livre para anexar erros de validação, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
    /// Metadados extras
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

fn default_type() -> String {
    "about:blank".to_string()
}
