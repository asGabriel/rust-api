use serde::Deserialize;

/// Identity asserted by a Google ID token, available only after its signature,
/// issuer, audience and expiration have been verified.
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleIdentity {
    pub sub: String,
    pub email: String,
    #[serde(default)]
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
}

impl GoogleIdentity {
    /// Normalized email used to match the allowlist.
    pub fn normalized_email(&self) -> String {
        self.email.trim().to_lowercase()
    }
}
