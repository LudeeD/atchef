use async_trait::async_trait;

use crate::Error;

/// Trait for providing authentication to API requests.
///
/// Implement this in your OAuth crate to provide DPoP-based authentication,
/// or use `BearerSession` for simple bearer token auth (app passwords, testing).
#[async_trait]
pub trait Session: Send + Sync {
    /// The authenticated user's DID.
    fn did(&self) -> &str;

    /// The user's PDS URL (e.g., "https://bsky.social").
    fn pds_url(&self) -> &str;

    /// Called before each request to get authorization headers.
    ///
    /// For DPoP: returns Authorization + DPoP headers.
    /// For app passwords: returns just the Authorization header.
    ///
    /// # Arguments
    /// * `method` - HTTP method ("GET", "POST", etc.)
    /// * `url` - Full request URL
    /// * `nonce` - Optional DPoP nonce (required by some servers)
    async fn get_auth_headers(
        &self,
        method: &str,
        url: &str,
        nonce: Option<&str>,
    ) -> Result<Vec<(String, String)>, Error>;
}

/// Simple bearer token session for testing or app passwords.
pub struct BearerSession {
    did: String,
    pds_url: String,
    access_token: String,
}

impl BearerSession {
    pub fn new(
        did: impl Into<String>,
        pds_url: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            did: did.into(),
            pds_url: pds_url.into(),
            access_token: access_token.into(),
        }
    }
}

#[async_trait]
impl Session for BearerSession {
    fn did(&self) -> &str {
        &self.did
    }

    fn pds_url(&self) -> &str {
        &self.pds_url
    }

    async fn get_auth_headers(
        &self,
        _method: &str,
        _url: &str,
        _nonce: Option<&str>,
    ) -> Result<Vec<(String, String)>, Error> {
        Ok(vec![(
            "Authorization".to_string(),
            format!("Bearer {}", self.access_token),
        )])
    }
}
