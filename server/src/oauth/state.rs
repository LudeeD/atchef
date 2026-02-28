use chrono::{DateTime, Utc};
use jsonwebtoken::jwk::Jwk;
use serde::{Deserialize, Serialize};

/// Stored in session during OAuth flow (before callback)
#[derive(Clone, Serialize, Deserialize)]
pub struct PendingAuth {
    pub state: String,
    pub code_verifier: String,
    pub dpop_private_key_pem: String,
    pub dpop_public_jwk: Jwk,
    pub authorization_server: String,
    pub token_endpoint: String,
    pub pds_url: String,
    pub handle: String,
    pub created_at: DateTime<Utc>,
}

/// Stored in session after successful auth
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub did: String,
    pub handle: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub dpop_private_key_pem: String,
    pub dpop_public_jwk: Jwk,
    pub pds_url: String,
    pub profile: Option<crate::models::ProfileRecord>,
}
