use std::time::{Duration, Instant};

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use tokio::sync::RwLock;

use crate::modules::auth::domain::google::GoogleIdentity;

const GOOGLE_CERTS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const GOOGLE_ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];
const DEFAULT_KEYS_TTL: Duration = Duration::from_secs(60 * 60);

/// Verifies Google ID tokens issued for this application's OAuth client.
#[async_trait]
pub trait GoogleTokenVerifier {
    async fn verify(&self, id_token: &str) -> HttpResult<GoogleIdentity>;
}

pub type DynGoogleTokenVerifier = dyn GoogleTokenVerifier + Send + Sync;

pub struct GoogleTokenVerifierImpl {
    http_client: reqwest::Client,
    client_id: String,
    keys: RwLock<Option<CachedKeys>>,
}

struct CachedKeys {
    jwks: JwkSet,
    expires_at: Instant,
}

impl GoogleTokenVerifierImpl {
    pub fn new(client_id: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            client_id,
            keys: RwLock::new(None),
        }
    }

    /// Returns the decoding key for `kid`, refetching Google's public keys
    /// when the cache expired or doesn't know the key (Google rotates them).
    async fn decoding_key(&self, kid: &str) -> HttpResult<DecodingKey> {
        {
            let cache = self.keys.read().await;
            if let Some(cached) = cache.as_ref().filter(|c| c.expires_at > Instant::now()) {
                if let Some(jwk) = cached.jwks.find(kid) {
                    return Self::key_from_jwk(jwk);
                }
            }
        }

        let fetched = self.fetch_keys().await?;
        let key = fetched
            .jwks
            .find(kid)
            .ok_or_else(|| Box::new(HttpError::unauthorized("Unknown Google signing key")))
            .and_then(Self::key_from_jwk);

        *self.keys.write().await = Some(fetched);
        key
    }

    async fn fetch_keys(&self) -> HttpResult<CachedKeys> {
        let response = self
            .http_client
            .get(GOOGLE_CERTS_URL)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| {
                Box::new(HttpError::bad_gateway(
                    "Failed to fetch Google signing keys",
                ))
            })?;

        let ttl = Self::max_age(response.headers()).unwrap_or(DEFAULT_KEYS_TTL);
        let jwks = response
            .json::<JwkSet>()
            .await
            .map_err(|_| Box::new(HttpError::bad_gateway("Invalid Google signing keys")))?;

        Ok(CachedKeys {
            jwks,
            expires_at: Instant::now() + ttl,
        })
    }

    fn max_age(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
        headers
            .get(reqwest::header::CACHE_CONTROL)?
            .to_str()
            .ok()?
            .split(',')
            .find_map(|directive| directive.trim().strip_prefix("max-age="))
            .and_then(|seconds| seconds.parse().ok())
            .map(Duration::from_secs)
    }

    fn key_from_jwk(jwk: &jsonwebtoken::jwk::Jwk) -> HttpResult<DecodingKey> {
        DecodingKey::from_jwk(jwk)
            .map_err(|_| Box::new(HttpError::bad_gateway("Invalid Google signing key")))
    }
}

#[async_trait]
impl GoogleTokenVerifier for GoogleTokenVerifierImpl {
    async fn verify(&self, id_token: &str) -> HttpResult<GoogleIdentity> {
        let header = decode_header(id_token)
            .map_err(|_| Box::new(HttpError::unauthorized("Invalid Google token")))?;
        let kid = header
            .kid
            .ok_or_else(|| Box::new(HttpError::unauthorized("Invalid Google token")))?;

        let key = self.decoding_key(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&GOOGLE_ISSUERS);

        let identity = decode::<GoogleIdentity>(id_token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|_| Box::new(HttpError::unauthorized("Invalid or expired Google token")))?;

        if !identity.email_verified {
            return Err(Box::new(HttpError::unauthorized(
                "Google account email is not verified",
            )));
        }

        Ok(identity)
    }
}
