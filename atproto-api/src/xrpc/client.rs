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

    pub async fn get<T: DeserializeOwned>(&self, nsid: &str, params: &[(&str, &str)]) -> Result<T, Error> {
        let url = self.build_url(nsid, params)?;
        let resp = self.with_dpop_retry("GET", &url, |headers| {
            apply_headers(self.http.get(url.clone()), headers)
        }).await?;
        parse_json(resp).await
    }

    pub async fn post<I: Serialize, O: DeserializeOwned>(&self, nsid: &str, body: &I) -> Result<O, Error> {
        let url = self.build_url(nsid, &[])?;
        let body_json = serde_json::to_value(body)
            .map_err(|e| Error::Internal(format!("Failed to serialize body: {}", e)))?;
        let resp = self.with_dpop_retry("POST", &url, |headers| {
            apply_headers(self.http.post(url.clone()).json(&body_json), headers)
        }).await?;
        parse_json(resp).await
    }

    pub async fn post_bytes<O: DeserializeOwned>(&self, nsid: &str, data: Vec<u8>, content_type: &str) -> Result<O, Error> {
        let url = self.build_url(nsid, &[])?;
        let resp = self.with_dpop_retry("POST", &url, |headers| {
            apply_headers(
                self.http.post(url.clone()).header("Content-Type", content_type).body(data.clone()),
                headers,
            )
        }).await?;
        parse_json(resp).await
    }

    pub async fn post_no_response<I: Serialize>(&self, nsid: &str, body: &I) -> Result<(), Error> {
        let url = self.build_url(nsid, &[])?;
        let body_json = serde_json::to_value(body)
            .map_err(|e| Error::Internal(format!("Failed to serialize body: {}", e)))?;
        let resp = self.with_dpop_retry("POST", &url, |headers| {
            apply_headers(self.http.post(url.clone()).json(&body_json), headers)
        }).await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(parse_error_response(resp).await)
        }
    }

    /// Sends a request, retrying once with a DPoP nonce if the server returns 401.
    ///
    /// `build` is called with auth headers and returns a ready-to-send `RequestBuilder`.
    /// It may be called twice (initial attempt + nonce retry), so captured data must be cloneable.
    async fn with_dpop_retry<F>(&self, method: &str, url: &Url, build: F) -> Result<reqwest::Response, Error>
    where
        F: Fn(Vec<(String, String)>) -> reqwest::RequestBuilder,
    {
        let headers = self.session.get_auth_headers(method, url.as_str(), None).await?;
        let resp = build(headers).send().await?;

        if resp.status() != 401 {
            return Ok(resp);
        }

        let nonce = resp.headers()
            .get("DPoP-Nonce")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        match nonce {
            Some(nonce) => {
                let headers = self.session.get_auth_headers(method, url.as_str(), Some(&nonce)).await?;
                Ok(build(headers).send().await?)
            }
            None => Ok(resp),
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

fn apply_headers(mut req: reqwest::RequestBuilder, headers: Vec<(String, String)>) -> reqwest::RequestBuilder {
    for (name, value) in headers {
        req = req.header(&name, &value);
    }
    req
}

async fn parse_json<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, Error> {
    if resp.status().is_success() {
        Ok(resp.json().await?)
    } else {
        Err(parse_error_response(resp).await)
    }
}

async fn parse_error_response(resp: reqwest::Response) -> Error {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    if let Ok(err) = serde_json::from_str::<XrpcErrorResponse>(&body) {
        Error::Xrpc { status, error: err.error, message: err.message }
    } else {
        Error::Xrpc { status, error: "Unknown".to_string(), message: Some(body) }
    }
}
