use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use crate::error::XrpcErrorResponse;
use crate::session::Session;
use crate::Error;

/// HTTP client for XRPC requests with session-based authentication.
pub struct XrpcClient<'a, S: Session> {
    session: &'a S,
    http: &'a Client,
}

impl<'a, S: Session> XrpcClient<'a, S> {
    pub fn new(session: &'a S, http: &'a Client) -> Self {
        Self { session, http }
    }

    /// Make a GET request to an XRPC endpoint.
    pub async fn get<T: DeserializeOwned>(
        &self,
        nsid: &str,
        params: &[(&str, &str)],
    ) -> Result<T, Error> {
        let url = self.build_url(nsid, params)?;

        let headers = self
            .session
            .get_auth_headers("GET", url.as_str(), None)
            .await?;

        let mut req = self.http.get(url);
        for (name, value) in headers {
            req = req.header(&name, &value);
        }

        let resp = req.send().await?;
        self.handle_response(resp).await
    }

    /// Make a POST request to an XRPC endpoint with JSON body.
    pub async fn post<I: Serialize, O: DeserializeOwned>(
        &self,
        nsid: &str,
        body: &I,
    ) -> Result<O, Error> {
        let url = self.build_url(nsid, &[])?;
        tracing::debug!("XRPC POST request: nsid={}, url={}", nsid, url);

        // First attempt without nonce
        let headers = self
            .session
            .get_auth_headers("POST", url.as_str(), None)
            .await?;

        let mut req = self.http.post(url.clone()).json(body);
        for (name, value) in headers {
            req = req.header(&name, &value);
        }

        tracing::debug!("XRPC POST sending request (first attempt)...");
        let resp = req.send().await?;
        let status = resp.status();
        tracing::debug!("XRPC POST response status: {}", status);

        // Check if server requires a DPoP nonce
        if status == 401 {
            // Extract DPoP-Nonce header if present
            let nonce = resp.headers()
                .get("DPoP-Nonce")
                .and_then(|v| v.to_str().ok());
            
            if let Some(nonce_str) = nonce {
                tracing::debug!("Server requires DPoP nonce: {}", nonce_str);
                
                // Serialize body for retry
                let body_json = serde_json::to_value(body)
                    .map_err(|e| Error::Internal(format!("Failed to serialize body: {}", e)))?;
                
                // Retry with nonce
                tracing::debug!("Retrying with DPoP nonce...");
                let headers_with_nonce = self
                    .session
                    .get_auth_headers("POST", url.as_str(), Some(nonce_str))
                    .await?;
                
                let mut retry_req = self.http.post(url).json(&body_json);
                for (name, value) in headers_with_nonce {
                    retry_req = retry_req.header(&name, &value);
                }
                
                let retry_resp = retry_req.send().await?;
                let retry_status = retry_resp.status();
                tracing::debug!("Retry response status: {}", retry_status);
                
                return self.handle_response_with_body(retry_resp, retry_status).await;
            }
            
            // No nonce header, return the error
            let body_text = resp.text().await.unwrap_or_default();
            let status_code = status.as_u16();
            
            if let Ok(err) = serde_json::from_str::<XrpcErrorResponse>(&body_text) {
                return Err(Error::Xrpc {
                    status: status_code,
                    error: err.error,
                    message: err.message,
                });
            }
            
            return Err(Error::Xrpc {
                status: status_code,
                error: "Unauthorized".to_string(),
                message: Some(body_text),
            });
        }

        self.handle_response_with_body(resp, status).await
    }

    async fn handle_response_with_body<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
        status: reqwest::StatusCode,
    ) -> Result<T, Error> {
        if status.is_success() {
            let body = resp.json().await?;
            Ok(body)
        } else {
            let status_code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();

            tracing::error!(
                "XRPC error response: status={}, body={}",
                status_code,
                body
            );

            if let Ok(err) = serde_json::from_str::<XrpcErrorResponse>(&body) {
                Err(Error::Xrpc {
                    status: status_code,
                    error: err.error,
                    message: err.message,
                })
            } else {
                Err(Error::Xrpc {
                    status: status_code,
                    error: "Unknown".to_string(),
                    message: Some(body),
                })
            }
        }
    }

    /// Make a POST request with raw bytes (for blob upload).
    pub async fn post_bytes<O: DeserializeOwned>(
        &self,
        nsid: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<O, Error> {
        let url = self.build_url(nsid, &[])?;

        let headers = self
            .session
            .get_auth_headers("POST", url.as_str(), None)
            .await?;

        let mut req = self
            .http
            .post(url)
            .header("Content-Type", content_type)
            .body(data);

        for (name, value) in headers {
            req = req.header(&name, &value);
        }

        let resp = req.send().await?;
        self.handle_response(resp).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, Error> {
        let status = resp.status();

        if status.is_success() {
            let body = resp.json().await?;
            Ok(body)
        } else {
            let status_code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();

            if let Ok(err) = serde_json::from_str::<XrpcErrorResponse>(&body) {
                Err(Error::Xrpc {
                    status: status_code,
                    error: err.error,
                    message: err.message,
                })
            } else {
                Err(Error::Xrpc {
                    status: status_code,
                    error: "Unknown".to_string(),
                    message: Some(body),
                })
            }
        }
    }

    /// Make a POST request that returns no content.
    pub async fn post_no_response<I: Serialize>(&self, nsid: &str, body: &I) -> Result<(), Error> {
        let url = self.build_url(nsid, &[])?;

        let headers = self
            .session
            .get_auth_headers("POST", url.as_str(), None)
            .await?;

        let mut req = self.http.post(url).json(body);
        for (name, value) in headers {
            req = req.header(&name, &value);
        }

        let resp = req.send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            if let Ok(err) = serde_json::from_str::<XrpcErrorResponse>(&body) {
                Err(Error::Xrpc {
                    status,
                    error: err.error,
                    message: err.message,
                })
            } else {
                Err(Error::Xrpc {
                    status,
                    error: "Unknown".to_string(),
                    message: Some(body),
                })
            }
        }
    }

    fn build_url(&self, nsid: &str, params: &[(&str, &str)]) -> Result<Url, Error> {
        let base = self.session.pds_url().trim_end_matches('/');
        let mut url = Url::parse(&format!("{}/xrpc/{}", base, nsid))?;

        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, value);
        }

        Ok(url)
    }


}
