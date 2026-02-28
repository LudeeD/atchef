use async_trait::async_trait;
use atproto_api::{Error as AtprotoError, Session};
use jsonwebtoken::jwk::Jwk;

use super::dpop;

/// DPoP-based session for authenticated ATProto API requests.
pub struct DpopSession {
    did: String,
    pds_url: String,
    access_token: String,
    dpop_private_key_pem: String,
    dpop_public_jwk: Jwk,
}

impl DpopSession {
    pub fn new(
        did: impl Into<String>,
        pds_url: impl Into<String>,
        access_token: impl Into<String>,
        dpop_private_key_pem: impl Into<String>,
        dpop_public_jwk: Jwk,
    ) -> Self {
        Self {
            did: did.into(),
            pds_url: pds_url.into(),
            access_token: access_token.into(),
            dpop_private_key_pem: dpop_private_key_pem.into(),
            dpop_public_jwk,
        }
    }
}

#[async_trait]
impl Session for DpopSession {
    fn did(&self) -> &str {
        &self.did
    }

    fn pds_url(&self) -> &str {
        &self.pds_url
    }

    async fn get_auth_headers(
        &self,
        method: &str,
        url: &str,
        nonce: Option<&str>,
    ) -> Result<Vec<(String, String)>, AtprotoError> {
        tracing::debug!(
            "Generating auth headers: method={}, url={}, pds_url={}, token_prefix={}, nonce={:?}",
            method,
            url,
            self.pds_url,
            &self.access_token[..self.access_token.len().min(20)],
            nonce
        );

        let dpop_proof = dpop::create_proof(
            &self.dpop_private_key_pem,
            &self.dpop_public_jwk,
            method,
            url,
            nonce,
            Some(&self.access_token),
        )
        .map_err(|e| AtprotoError::Session(e.to_string()))?;

        tracing::debug!("Auth headers generated successfully");

        Ok(vec![
            ("Authorization".to_string(), format!("DPoP {}", self.access_token)),
            ("DPoP".to_string(), dpop_proof),
        ])
    }
}
