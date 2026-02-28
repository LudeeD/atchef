use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[allow(dead_code)]
    pub pushed_authorization_request_endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DidDocument {
    #[allow(dead_code)]
    id: String,
    service: Option<Vec<DidService>>,
}

#[derive(Debug, Deserialize)]
struct DidService {
    #[allow(dead_code)]
    id: String,
    #[serde(rename = "type")]
    service_type: String,
    #[serde(rename = "serviceEndpoint")]
    service_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct ResolveHandleResponse {
    did: String,
}

/// Resolve a handle to a DID
pub async fn resolve_handle(client: &reqwest::Client, handle: &str) -> Result<String> {
    // Try HTTPS method first (works for custom domain handles)
    let https_url = format!("https://{}/.well-known/atproto-did", handle);
    if let Ok(response) = client.get(&https_url).send().await {
        if response.status().is_success() {
            if let Ok(text) = response.text().await {
                let did = text.trim().to_string();
                if did.starts_with("did:") {
                    return Ok(did);
                }
            }
        }
    }

    // Fall back to Bluesky public API (works for *.bsky.social handles)
    let api_url = format!(
        "https://public.api.bsky.app/xrpc/com.atproto.identity.resolveHandle?handle={}",
        urlencoding::encode(handle)
    );
    let response = client
        .get(&api_url)
        .send()
        .await
        .context("failed to resolve handle")?;

    if !response.status().is_success() {
        return Err(anyhow!("handle resolution failed: {}", response.status()));
    }

    let data: ResolveHandleResponse = response.json().await.context("failed to parse response")?;
    Ok(data.did)
}

/// Get the PDS URL from a DID document
pub async fn get_pds_url(client: &reqwest::Client, did: &str) -> Result<String> {
    let doc = fetch_did_document(client, did).await?;

    let services = doc.service.ok_or_else(|| anyhow!("no services in DID document"))?;

    for service in services {
        if service.service_type == "AtprotoPersonalDataServer" {
            return Ok(service.service_endpoint);
        }
    }

    Err(anyhow!("no PDS service found in DID document"))
}

async fn fetch_did_document(client: &reqwest::Client, did: &str) -> Result<DidDocument> {
    let url = if did.starts_with("did:plc:") {
        format!("https://plc.directory/{}", did)
    } else if did.starts_with("did:web:") {
        let domain = did.strip_prefix("did:web:").unwrap();
        format!("https://{}/.well-known/did.json", domain)
    } else {
        return Err(anyhow!("unsupported DID method"));
    };

    let response = client
        .get(&url)
        .send()
        .await
        .context("failed to fetch DID document")?;

    if !response.status().is_success() {
        return Err(anyhow!("DID document fetch failed: {}", response.status()));
    }

    response.json().await.context("failed to parse DID document")
}

#[derive(Debug, Deserialize)]
struct ProtectedResourceMetadata {
    authorization_servers: Vec<String>,
}

/// Get authorization server metadata from a PDS
pub async fn get_auth_server_metadata(
    client: &reqwest::Client,
    pds_url: &str,
) -> Result<AuthServerMetadata> {
    // First, get the protected resource metadata to find the authorization server
    let pr_url = format!(
        "{}/.well-known/oauth-protected-resource",
        pds_url.trim_end_matches('/')
    );

    tracing::debug!("Fetching protected resource metadata from: {}", pr_url);

    let response = client
        .get(&pr_url)
        .send()
        .await
        .context("failed to fetch protected resource metadata")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "protected resource metadata fetch failed: {} (url: {})",
            response.status(),
            pr_url
        ));
    }

    let pr_metadata: ProtectedResourceMetadata = response
        .json()
        .await
        .context("failed to parse protected resource metadata")?;

    let as_url = pr_metadata
        .authorization_servers
        .first()
        .ok_or_else(|| anyhow!("no authorization servers found"))?;

    tracing::debug!("Authorization server URL: {}", as_url);

    // Now fetch the authorization server metadata
    let as_metadata_url = format!(
        "{}/.well-known/oauth-authorization-server",
        as_url.trim_end_matches('/')
    );

    tracing::debug!("Fetching AS metadata from: {}", as_metadata_url);

    let response = client
        .get(&as_metadata_url)
        .send()
        .await
        .context("failed to fetch AS metadata")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "AS metadata fetch failed: {} (url: {})",
            response.status(),
            as_metadata_url
        ));
    }

    response.json().await.context("failed to parse AS metadata")
}
