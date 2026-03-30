use atproto_api::Agent;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Form, Json,
};
use maud::Markup;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use jsonwebtoken::jwk::Jwk;

use crate::lexicons::eu::atchef::recipe::RecordData;
use crate::models::{Recipe, RecipeDetail, ProfileRecord};
use crate::oauth::{discovery, dpop, pkce, AuthenticatedUser, DpopSession, PendingAuth};
use crate::views::{base_layout, base_layout_with_user, login_page, recipe_form_page, recipe_list, recipe_page};
use crate::{AppState, db};

const PENDING_AUTH_KEY: &str = "pending_auth";
const USER_KEY: &str = "user";

// Image upload configuration
const MAX_IMAGE_SIZE_BYTES: usize = 1024 * 1024; // 1MB
const ALLOWED_IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

/// Convert from our atproto_api::BlobRef to atrium_api::types::BlobRef
fn convert_blob_ref(blob_ref: &atproto_api::BlobRef) -> anyhow::Result<atrium_api::types::BlobRef> {
    use atrium_api::types::{BlobRef, TypedBlobRef, Blob};
    use std::str::FromStr;
    
    let cid = ipld_core::cid::Cid::from_str(blob_ref.cid())
        .map_err(|e| anyhow::anyhow!("Invalid CID: {}", e))?;
    
    Ok(BlobRef::Typed(TypedBlobRef::Blob(Blob {
        r#ref: atrium_api::types::CidLink(cid),
        mime_type: blob_ref.mime_type.clone(),
        size: blob_ref.size as usize,
    })))
}

async fn refresh_access_token(
    client: &reqwest::Client,
    token_endpoint: &str,
    dpop_private_key_pem: &str,
    dpop_public_jwk: &Jwk,
    client_id: &str,
    refresh_token: &str,
) -> anyhow::Result<TokenResponse> {
    // First attempt without nonce
    let dpop_proof = dpop::create_proof(
        dpop_private_key_pem,
        dpop_public_jwk,
        "POST",
        token_endpoint,
        None,
        None,
    )?;

    let response = client
        .post(token_endpoint)
        .header("DPoP", &dpop_proof)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ])
        .send()
        .await?;

    // Check if server requires a nonce
    if response.status() == 400 {
        if let Some(nonce) = response.headers().get("dpop-nonce") {
            let nonce_str = nonce.to_str().unwrap_or_default();
            tracing::debug!("Retrying token refresh with DPoP nonce: {}", nonce_str);

            // Retry with nonce
            let dpop_proof_with_nonce = dpop::create_proof(
                dpop_private_key_pem,
                dpop_public_jwk,
                "POST",
                token_endpoint,
                Some(nonce_str),
                None,
            )?;

            let retry_response = client
                .post(token_endpoint)
                .header("DPoP", &dpop_proof_with_nonce)
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh_token),
                    ("client_id", client_id),
                ])
                .send()
                .await?;

            if !retry_response.status().is_success() {
                let status = retry_response.status();
                let body = retry_response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("token refresh failed: {} - {}", status, body));
            }

            return retry_response.json().await.map_err(Into::into);
        }

        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("token refresh failed: 400 - {}", body));
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("token refresh failed: {} - {}", status, body));
    }

    response.json().await.map_err(Into::into)
}

async fn exchange_token(
    client: &reqwest::Client,
    token_endpoint: &str,
    dpop_private_key_pem: &str,
    dpop_public_jwk: &Jwk,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> anyhow::Result<TokenResponse> {
    // First attempt without nonce
    let dpop_proof = dpop::create_proof(
        dpop_private_key_pem,
        dpop_public_jwk,
        "POST",
        token_endpoint,
        None,
        None,
    )?;

    let response = client
        .post(token_endpoint)
        .header("DPoP", &dpop_proof)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await?;

    // Check if server requires a nonce
    if response.status() == 400 {
        if let Some(nonce) = response.headers().get("dpop-nonce") {
            let nonce_str = nonce.to_str().unwrap_or_default();
            tracing::debug!("Retrying token exchange with DPoP nonce: {}", nonce_str);

            // Retry with nonce
            let dpop_proof_with_nonce = dpop::create_proof(
                dpop_private_key_pem,
                dpop_public_jwk,
                "POST",
                token_endpoint,
                Some(nonce_str),
                None,
            )?;

            let retry_response = client
                .post(token_endpoint)
                .header("DPoP", &dpop_proof_with_nonce)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                    ("client_id", client_id),
                    ("code_verifier", code_verifier),
                ])
                .send()
                .await?;

            if !retry_response.status().is_success() {
                let status = retry_response.status();
                let body = retry_response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("token exchange failed: {} - {}", status, body));
            }

            return retry_response.json().await.map_err(Into::into);
        }

        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("token exchange failed: 400 - {}", body));
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("token exchange failed: {} - {}", status, body));
    }

    response.json().await.map_err(Into::into)
}

pub async fn home(State(state): State<AppState>, session: Session) -> Markup {
    let db_recipes = db::get_all_recipes(&state.sqlite_pool)
        .await
        .unwrap_or_default();
    
    let mut recipes = Vec::new();
    for row in &db_recipes {
        let author_info = crate::models::AuthorInfo::basic(row.author_handle.clone());
        recipes.push(Recipe::from_db_row(row, author_info));
    }

    let user = session
        .get::<AuthenticatedUser>(USER_KEY)
        .await
        .ok()
        .flatten();

    let content = recipe_list(&recipes, user.as_ref());
    let user_handle = user.map(|u| u.handle);
    base_layout_with_user("AtChef", content, user_handle.as_deref())
}

pub async fn profile(session: Session) -> Response {
    match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(user)) => Redirect::to(&format!("/profile/{}", user.handle)).into_response(),
        _ => Redirect::to("/login").into_response(),
    }
}

#[derive(Deserialize)]
struct ListRecordsValue {
    name: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Deserialize)]
struct ListRecordsRecord {
    uri: String,
    value: ListRecordsValue,
}

#[derive(Deserialize)]
struct ListRecordsResponse {
    records: Vec<ListRecordsRecord>,
}

#[derive(Deserialize)]
struct ProfileRecordResponse {
    value: crate::models::ProfileRecord,
}

#[derive(Deserialize)]
struct GetRecordValue {
    name: String,
    content: String,
    portions: u64,
    time: u64,
    #[serde(rename = "createdAt")]
    created_at: String,
    description: Option<String>,
    #[serde(rename = "prepTime")]
    prep_time: Option<u64>,
    #[serde(rename = "cookTime")]
    cook_time: Option<u64>,
    image: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetRecordResponse {
    value: GetRecordValue,
}

fn time_ago(created_at: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created_at) else {
        return "recently".to_string();
    };
    let secs = (chrono::Utc::now() - dt.to_utc()).num_seconds();
    match secs {
        s if s < 60 => "just now".to_string(),
        s if s < 3600 => format!("{} min ago", s / 60),
        s if s < 86400 => format!("{} hours ago", s / 3600),
        s => format!("{} days ago", s / 86400),
    }
}

pub async fn recipe(
    State(state): State<AppState>,
    session: Session,
    Path((handle, rkey)): Path<(String, String)>,
) -> Markup {
    let user = session.get::<AuthenticatedUser>(USER_KEY).await.ok().flatten();
    let result = async {
        // Cache-first: try DB before hitting PDS
        if let Ok(Some(row)) = db::get_recipe(&state.sqlite_pool, &handle, &rkey).await {
            let author_info = crate::models::AuthorInfo::basic(row.author_handle.clone());
            
            return Ok(RecipeDetail {
                id: row.rkey.clone(),
                name: row.name,
                content: row.content,
                portions: row.portions,
                time: row.time,
                author: author_info,
                time_ago: time_ago(&row.created_at.to_rfc3339()),
                comments: vec![],
                description: row.description,
                prep_time: row.prep_time,
                cook_time: row.cook_time,
                image_cid: row.image_cid,
                image_mime_type: row.image_mime_type,
            });
        }

        let did = discovery::resolve_handle(&state.http_client, &handle).await?;
        let pds_url = discovery::get_pds_url(&state.http_client, &did).await?;
        let url = format!(
            "{}/xrpc/com.atproto.repo.getRecord?repo={}&collection=eu.atchef.recipe&rkey={}",
            pds_url.trim_end_matches('/'),
            urlencoding::encode(&did),
            urlencoding::encode(&rkey),
        );
        let response = state.http_client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("record not found: {}", response.status()));
        }
        let record: GetRecordResponse = response.json().await?;

        let author_info = crate::models::AuthorInfo::basic(handle.clone());
        
        let recipe_detail = RecipeDetail {
            id: rkey.clone(),
            name: record.value.name.clone(),
            content: record.value.content.clone(),
            portions: record.value.portions as u32,
            time: record.value.time as u32,
            author: author_info,
            time_ago: time_ago(&record.value.created_at),
            comments: vec![],
            description: record.value.description.clone(),
            prep_time: record.value.prep_time.map(|v| v as u32),
            cook_time: record.value.cook_time.map(|v| v as u32),
            image_cid: record.value.image.as_ref()
                .and_then(|img| img.get("ref"))
                .and_then(|r| r.get("$link"))
                .and_then(|cid| cid.as_str())
                .map(String::from),
            image_mime_type: record.value.image.as_ref()
                .and_then(|img| img.get("mimeType"))
                .and_then(|mime| mime.as_str())
                .map(String::from),
        };

        let uri = format!("at://{}/eu.atchef.recipe/{}", did, rkey);
        let _ = db::save_recipe(
            &state.sqlite_pool,
            &uri,
            &did,
            &handle,
            &rkey,
            &recipe_detail.name,
            &recipe_detail.content,
            recipe_detail.portions,
            recipe_detail.time,
            &record.value.created_at,
            recipe_detail.description.as_deref(),
            recipe_detail.prep_time,
            recipe_detail.cook_time,
            record.value.image.as_ref()
                .and_then(|img| img.get("ref"))
                .and_then(|r| r.get("$link"))
                .and_then(|cid| cid.as_str()),
            record.value.image.as_ref()
                .and_then(|img| img.get("mimeType"))
                .and_then(|mime| mime.as_str()),
        )
        .await;

        Ok(recipe_detail)
    }
    .await;

    match result {
        Ok(detail) => {
            let content = recipe_page(&detail);
            base_layout_with_user(&format!("{} | AtChef", detail.name), content, user.as_ref().map(|u| u.handle.as_str()))
        }
        Err(e) => {
            tracing::error!("Failed to load recipe {}/{}: {}", handle, rkey, e);
            base_layout(
                "Not Found | AtChef",
                maud::html! {
                    h1 { "Recipe not found" }
                    p { "The recipe you're looking for doesn't exist." }
                    p { a href="/" { "Back to home" } }
                },
            )
        }
    }
}

pub async fn public_profile(
    State(state): State<AppState>,
    Path(handle): Path<String>,
    session: Session,
) -> Markup {
    let viewer = session
        .get::<AuthenticatedUser>(USER_KEY)
        .await
        .ok()
        .flatten();
    let is_owner = viewer
        .as_ref()
        .map(|u| u.handle == handle)
        .unwrap_or(false);

    let result = async {
        let did = discovery::resolve_handle(&state.http_client, &handle).await?;
        let pds_url = discovery::get_pds_url(&state.http_client, &did).await?;

        // Fetch profile record (best-effort)
        let profile_url = format!(
            "{}/xrpc/com.atproto.repo.getRecord?repo={}&collection=app.bsky.actor.profile&rkey=self",
            pds_url.trim_end_matches('/'),
            urlencoding::encode(&did),
        );
        let profile = if let Ok(resp) = state.http_client.get(&profile_url).send().await {
            if resp.status().is_success() {
                resp.json::<ProfileRecordResponse>().await.ok().map(|r| r.value)
            } else {
                None
            }
        } else {
            None
        };

        let avatar_url = profile.as_ref().and_then(|p| p.avatar.as_ref()).and_then(|blob| {
            blob.reference["$link"].as_str().map(|cid| {
                format!(
                    "{}/xrpc/com.atproto.sync.getBlob?did={}&cid={}",
                    pds_url.trim_end_matches('/'),
                    urlencoding::encode(&did),
                    urlencoding::encode(cid),
                )
            })
        });
        let display_name = profile.as_ref().and_then(|p| p.display_name.clone());
        let description = profile.as_ref().and_then(|p| p.description.clone());

        let url = format!(
            "{}/xrpc/com.atproto.repo.listRecords?repo={}&collection=eu.atchef.recipe",
            pds_url.trim_end_matches('/'),
            urlencoding::encode(&did),
        );
        let response = state.http_client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("failed to list records: {}", response.status()));
        }
        let list: ListRecordsResponse = response.json().await?;
        let recipes = list.records.into_iter().map(|r| {
            let rkey = r.uri.split('/').last().unwrap_or("").to_string();
            let author_info = crate::models::AuthorInfo::basic(handle.clone());
            crate::models::Recipe {
                id: rkey,
                name: r.value.name,
                author: author_info,
                time_ago: time_ago(&r.value.created_at),
                comment_count: 0,
            }
        }).collect::<Vec<_>>();
        let is_member = db::is_atchef_member(&state.sqlite_pool, &did).await.unwrap_or(false);
        Ok((recipes, display_name, description, avatar_url, is_member))
    }
    .await;

    match result {
        Ok((recipes, display_name, description, avatar_url, is_member)) => {
            let content = crate::views::public_profile_page(
                &handle,
                &recipes,
                is_owner,
                display_name.as_deref(),
                description.as_deref(),
                avatar_url.as_deref(),
                is_member,
            );
            base_layout_with_user(
                &format!("{} | AtChef", handle),
                content,
                viewer.as_ref().map(|u| u.handle.as_str()),
            )
        }
        Err(e) => {
            tracing::error!("Failed to load profile {}: {}", handle, e);
            base_layout(
                "Not Found | AtChef",
                maud::html! {
                    h1 { "Profile not found" }
                    p { a href="/" { "Back to home" } }
                },
            )
        }
    }
}

pub async fn login_page_handler(session: Session) -> Markup {
    if let Ok(Some(_)) = session.get::<AuthenticatedUser>(USER_KEY).await {
        return base_layout(
            "Already signed in | AtChef",
            maud::html! {
                h1 { "Already signed in" }
                p { "You are already signed in." }
                form method="post" action="/logout" {
                    button type="submit" { "Sign out" }
                }
            },
        );
    }

    let content = login_page(None);
    base_layout("Sign in | AtChef", content)
}

#[derive(Deserialize)]
pub struct LoginForm {
    handle: String,
}

pub async fn login_start(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Response {
    let handle = form.handle.trim().to_lowercase();

    let result = async {
        let did = discovery::resolve_handle(&state.http_client, &handle).await?;
        let pds_url = discovery::get_pds_url(&state.http_client, &did).await?;
        let as_metadata = discovery::get_auth_server_metadata(&state.http_client, &pds_url).await?;

        let pkce = pkce::generate();
        let dpop_keypair = dpop::generate_keypair()?;
        let oauth_state = uuid::Uuid::new_v4().to_string();

        let pending = PendingAuth {
            state: oauth_state.clone(),
            code_verifier: pkce.verifier,
            dpop_private_key_pem: dpop_keypair.private_key_pem,
            dpop_public_jwk: dpop_keypair.public_jwk,
            authorization_server: as_metadata.issuer.clone(),
            token_endpoint: as_metadata.token_endpoint.clone(),
            pds_url: pds_url.clone(),
            handle: handle.clone(),
            created_at: chrono::Utc::now(),
        };

        session
            .insert(PENDING_AUTH_KEY, pending)
            .await
            .map_err(|e| anyhow::anyhow!("session error: {}", e))?;

        let par_endpoint = as_metadata.pushed_authorization_request_endpoint
            .ok_or_else(|| anyhow::anyhow!("authorization server does not support PAR"))?;

        let redirect_uri = format!("{}/oauth/callback", state.base_url);

        let par_response = state.http_client
            .post(&par_endpoint)
            .form(&[
                ("response_type", "code"),
                ("client_id", state.client_id.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("state", oauth_state.as_str()),
                ("code_challenge", pkce.challenge.as_str()),
                ("code_challenge_method", "S256"),
                ("scope", "atproto transition:generic"),
                ("login_hint", handle.as_str()),
            ])
            .send()
            .await?;

        if !par_response.status().is_success() {
            let status = par_response.status();
            let body = par_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("PAR request failed: {} - {}", status, body));
        }

        #[derive(Deserialize)]
        struct PARResponse { request_uri: String }
        let par_data: PARResponse = par_response.json().await?;

        let auth_url = format!(
            "{}?client_id={}&request_uri={}",
            as_metadata.authorization_endpoint,
            urlencoding::encode(&state.client_id),
            urlencoding::encode(&par_data.request_uri),
        );

        Ok::<_, anyhow::Error>(auth_url)
    }
    .await;

    match result {
        Ok(auth_url) => Redirect::to(&auth_url).into_response(),
        Err(e) => {
            tracing::error!("OAuth error: {}", e);
            let content = login_page(Some(&format!("Login failed: {}", e)));
            base_layout("Sign in | AtChef", content).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<CallbackParams>,
) -> Response {
    if let Some(error) = params.error {
        let desc = params.error_description.unwrap_or_default();
        let content = login_page(Some(&format!("Authorization failed: {} - {}", error, desc)));
        return base_layout("Sign in | AtChef", content).into_response();
    }

    let code = match params.code {
        Some(c) => c,
        None => {
            let content = login_page(Some("Missing authorization code"));
            return base_layout("Sign in | AtChef", content).into_response();
        }
    };

    let result = async {
        let pending: PendingAuth = session
            .get(PENDING_AUTH_KEY)
            .await
            .map_err(|e| anyhow::anyhow!("session error: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("no pending auth found"))?;

        if params.state.as_deref() != Some(&pending.state) {
            return Err(anyhow::anyhow!("state mismatch"));
        }

        let redirect_uri = format!("{}/oauth/callback", state.base_url);

        // Token exchange with DPoP nonce handling
        let tokens = exchange_token(
            &state.http_client,
            &pending.token_endpoint,
            &pending.dpop_private_key_pem,
            &pending.dpop_public_jwk,
            &state.client_id,
            &code,
            &redirect_uri,
            &pending.code_verifier,
        )
        .await?;

        // Fetch the user's profile record
        let dpop_session = DpopSession::new(
            &tokens.sub,
            &pending.authorization_server,
            &tokens.access_token,
            &pending.dpop_private_key_pem,
            pending.dpop_public_jwk.clone(),
        );
        let agent = Agent::with_http_client(dpop_session, state.http_client.clone());
        let profile = agent
            .repo()
            .get_record::<ProfileRecord>(&tokens.sub, "app.bsky.actor.profile", "self")
            .await
            .ok()
            .map(|r| r.value); // Gracefully handle errors - profile is optional

        let user = AuthenticatedUser {
            did: tokens.sub,
            handle: pending.handle,
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(tokens.expires_in as i64),
            dpop_private_key_pem: pending.dpop_private_key_pem,
            dpop_public_jwk: pending.dpop_public_jwk,
            pds_url: pending.pds_url,
            profile,
        };

        // Track user in database
        if let Err(e) = db::upsert_user(&state.sqlite_pool, &user.did, &user.handle).await {
            tracing::error!("Failed to track user: {}", e);
        }

        session.remove::<PendingAuth>(PENDING_AUTH_KEY).await.ok();
        session
            .insert(USER_KEY, user)
            .await
            .map_err(|e| anyhow::anyhow!("session error: {}", e))?;

        Ok(())
    }
    .await;

    match result {
        Ok(()) => Redirect::to("/").into_response(),
        Err(e) => {
            tracing::error!("OAuth callback error: {}", e);
            let content = login_page(Some(&format!("Login failed: {}", e)));
            base_layout("Sign in | AtChef", content).into_response()
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    sub: String,
}

pub async fn logout(session: Session) -> Redirect {
    session.remove::<AuthenticatedUser>(USER_KEY).await.ok();
    Redirect::to("/")
}

#[derive(Serialize)]
pub struct ClientMetadata {
    client_id: String,
    client_name: String,
    client_uri: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    response_types: Vec<String>,
    scope: String,
    token_endpoint_auth_method: String,
    application_type: String,
    dpop_bound_access_tokens: bool,
}

pub async fn client_metadata(State(state): State<AppState>) -> Json<ClientMetadata> {
    Json(ClientMetadata {
        client_id: state.client_id.clone(),
        client_name: "AtChef".to_string(),
        client_uri: state.base_url.clone(),
        redirect_uris: vec![format!("{}/oauth/callback", state.base_url)],
        grant_types: vec!["authorization_code".to_string(), "refresh_token".to_string()],
        response_types: vec!["code".to_string()],
        scope: "atproto transition:generic".to_string(),
        token_endpoint_auth_method: "none".to_string(),
        application_type: "web".to_string(),
        dpop_bound_access_tokens: true,
    })
}

// Removed RecipeForm - replaced with multipart parsing

#[derive(Debug)]
pub struct RecipeFormData {
    name: String,
    description: String,
    portions: u64,
    prep_time: u64,
    cook_time: u64,
    content: String,
    image: Option<(Vec<u8>, String)>, // (data, mime_type)
    post_to_bluesky: bool,
}

pub async fn new_recipe_form(session: Session) -> Response {
    match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(user)) => {
            let content = recipe_form_page(None);
            base_layout_with_user("New Recipe | AtChef", content, Some(&user.handle)).into_response()
        }
        _ => Redirect::to("/login").into_response(),
    }
}

async fn parse_recipe_multipart(mut multipart: Multipart) -> anyhow::Result<RecipeFormData> {
    let mut name = String::new();
    let mut description = String::new();
    let mut portions: u64 = 4;
    let mut prep_time: u64 = 15;
    let mut cook_time: u64 = 30;
    let mut content = String::new();
    let mut image: Option<(Vec<u8>, String)> = None;
    let mut post_to_bluesky = false;

    while let Some(field) = multipart.next_field().await? {
        let field_name = field.name().unwrap_or("").to_string();
        
        match field_name.as_str() {
            "name" => {
                name = field.text().await?;
            }
            "description" => {
                description = field.text().await?;
            }
            "portions" => {
                if let Ok(val) = field.text().await?.parse::<u64>() {
                    portions = val;
                }
            }
            "prep_time" => {
                if let Ok(val) = field.text().await?.parse::<u64>() {
                    prep_time = val;
                }
            }
            "cook_time" => {
                if let Ok(val) = field.text().await?.parse::<u64>() {
                    cook_time = val;
                }
            }
            "content" => {
                content = field.text().await?;
            }
            "recipe-image" => {
                if let Some(file_name) = field.file_name() {
                    if !file_name.is_empty() {
                        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
                        
                        // Validate content type
                        if !ALLOWED_IMAGE_TYPES.contains(&content_type.as_str()) {
                            return Err(anyhow::anyhow!("Invalid image type. Only PNG, JPEG, and WebP are allowed"));
                        }
                        
                        let data = field.bytes().await?;
                        
                        // Validate file size
                        if data.len() > MAX_IMAGE_SIZE_BYTES {
                            return Err(anyhow::anyhow!("Image file too large. Maximum size is {}MB", MAX_IMAGE_SIZE_BYTES / 1024 / 1024));
                        }
                        
                        image = Some((data.to_vec(), content_type));
                    }
                }
            }
            "post_to_bluesky" => {
                post_to_bluesky = field.text().await.map(|v| v == "1").unwrap_or(false);
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    if name.trim().is_empty() {
        return Err(anyhow::anyhow!("Recipe name is required"));
    }
    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Recipe content is required"));
    }

    Ok(RecipeFormData {
        name: name.trim().to_string(),
        description: description.trim().to_string(),
        portions,
        prep_time,
        cook_time,
        content: content.trim().to_string(),
        image,
        post_to_bluesky,
    })
}

pub async fn create_recipe(
    State(state): State<AppState>,
    session: Session,
    multipart: Multipart,
) -> Response {
    let mut user = match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(user)) => user,
        _ => return Redirect::to("/login").into_response(),
    };

    // Parse the multipart form data
    let form = match parse_recipe_multipart(multipart).await {
        Ok(form) => form,
        Err(e) => {
            tracing::error!("Failed to parse form data: {}", e);
            let content = recipe_form_page(Some(&format!("Invalid form data: {}", e)));
            return base_layout_with_user("New Recipe | AtChef", content, Some(&user.handle)).into_response();
        }
    };

    let post_to_bluesky = form.post_to_bluesky;

    let result = async {
        // Check if token is expired and refresh if needed
        let now = chrono::Utc::now();
        if now >= user.expires_at {
            tracing::info!("Access token expired, attempting refresh");
            if let Some(refresh_token) = &user.refresh_token {
                let metadata = discovery::get_auth_server_metadata(&state.http_client, &user.pds_url).await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch OAuth metadata: {}", e))?;
                
                let tokens = refresh_access_token(
                    &state.http_client,
                    &metadata.token_endpoint,
                    &user.dpop_private_key_pem,
                    &user.dpop_public_jwk,
                    &state.client_id,
                    refresh_token,
                ).await?;

                // Update user with new tokens
                user.access_token = tokens.access_token;
                user.refresh_token = tokens.refresh_token.or(user.refresh_token.clone());
                user.expires_at = chrono::Utc::now() + chrono::Duration::seconds(tokens.expires_in as i64);
                
                // Update session with refreshed tokens
                session.insert(USER_KEY, &user).await
                    .map_err(|e| anyhow::anyhow!("Failed to update session: {}", e))?;
                
                tracing::info!("Token refreshed successfully");
            } else {
                return Err(anyhow::anyhow!("Token expired and no refresh token available"));
            }
        }

        let dpop_session = DpopSession::new(
            &user.did,
            &user.pds_url,
            &user.access_token,
            &user.dpop_private_key_pem,
            user.dpop_public_jwk.clone(),
        );
        let agent = Agent::with_http_client(dpop_session, state.http_client.clone());

        let created_at = atrium_api::types::string::Datetime::now();
        // Already have form.name available, no need for separate variable

        let portions = form.portions.max(1);
        let time = (form.prep_time + form.cook_time).max(1);
        let description = if form.description.trim().is_empty() { None } else { Some(form.description.trim().to_string()) };
        let prep_time = if form.prep_time > 0 { Some(form.prep_time) } else { None };
        let cook_time = if form.cook_time > 0 { Some(form.cook_time) } else { None };

        // Handle image upload if present
        let image_blob = if let Some((image_data, mime_type)) = form.image {
            tracing::info!("Uploading image blob, size: {} bytes, type: {}", image_data.len(), mime_type);
            match agent.repo().upload_blob(image_data, &mime_type).await {
                Ok(blob_ref) => {
                    tracing::info!("Image uploaded successfully with CID: {}", blob_ref.cid());
                    Some(blob_ref)
                }
                Err(e) => {
                    tracing::error!("Failed to upload image: {}", e);
                    return Err(anyhow::anyhow!("Failed to upload image: {}", e));
                }
            }
        } else {
            None
        };

        let converted_image = if let Some(ref blob) = image_blob {
            Some(convert_blob_ref(blob)?)
        } else {
            None
        };

        let record = RecordData {
            name: form.name.clone(),
            description,
            portions: std::num::NonZeroU64::new(portions).unwrap(),
            time: std::num::NonZeroU64::new(time).unwrap(),
            prep_time,
            cook_time,
            content: form.content.clone(),
            image: converted_image,
            created_at: created_at.clone(),
        };

        let output = agent
            .repo()
            .create_record(&user.did, "eu.atchef.recipe", &record)
            .await?;

        Ok::<_, anyhow::Error>((output, created_at, form.name, record.content, portions as u32, time as u32, record.description, record.prep_time, record.cook_time, image_blob))
    }
    .await;

    match result {
        Ok((output, created_at, recipe_name, content, portions, time, description, prep_time, cook_time, original_blob)) => {
            let rkey = output.uri.split('/').last().unwrap_or("").to_string();
            let uri = output.uri.clone();

            // Save recipe to local database for caching
            if let Err(e) = db::save_recipe(
                &state.sqlite_pool,
                &uri,
                &user.did,
                &user.handle,
                &rkey,
                &recipe_name,
                &content,
                portions,
                time,
                created_at.as_str(),
                description.as_deref(),
                prep_time.map(|v| v as u32),
                cook_time.map(|v| v as u32),
                original_blob.as_ref().map(|img| img.cid()),
                original_blob.as_ref().map(|img| img.mime_type.as_str()),
            ).await {
                tracing::error!("Failed to save recipe to local database cache: {}", e);
                // Recipe was successfully created in AT Protocol, but local caching failed
                // This is non-critical - the recipe will still be accessible via AT Protocol
                // and will be cached when accessed through the recipe view
            }

            if post_to_bluesky {
                let recipe_url = format!("{}/profile/{}/recipe/{}", state.base_url, user.handle, rkey);
                if let Err(e) = post_recipe_to_bluesky(&user, &state, &recipe_name, &recipe_url).await {
                    tracing::error!("Failed to post to Bluesky: {}", e);
                }
            }

            Redirect::to(&format!("/profile/{}/recipe/{}", user.handle, rkey)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create recipe: {}", e);
            let content = recipe_form_page(Some(&format!("Failed to create recipe: {}", e)));
            base_layout_with_user("New Recipe | AtChef", content, Some(&user.handle)).into_response()
        }
    }
}

async fn post_recipe_to_bluesky(
    user: &AuthenticatedUser,
    state: &AppState,
    recipe_name: &str,
    recipe_url: &str,
) -> anyhow::Result<()> {
    let dpop_session = DpopSession::new(
        &user.did,
        &user.pds_url,
        &user.access_token,
        &user.dpop_private_key_pem,
        user.dpop_public_jwk.clone(),
    );
    let agent = Agent::with_http_client(dpop_session, state.http_client.clone());

    let text = format!("New recipe: {}\n\n{}", recipe_name, recipe_url);
    let url_start = text.len() - recipe_url.len();
    let url_end = text.len();

    #[derive(Serialize)]
    struct ByteSlice { #[serde(rename = "byteStart")] byte_start: usize, #[serde(rename = "byteEnd")] byte_end: usize }
    #[derive(Serialize)]
    struct LinkFeature { #[serde(rename = "$type")] t: &'static str, uri: String }
    #[derive(Serialize)]
    struct Facet { index: ByteSlice, features: Vec<LinkFeature> }
    #[derive(Serialize)]
    struct BskyPost {
        #[serde(rename = "$type")] t: &'static str,
        text: String,
        facets: Vec<Facet>,
        #[serde(rename = "createdAt")] created_at: String,
    }

    let post = BskyPost {
        t: "app.bsky.feed.post",
        text,
        facets: vec![Facet {
            index: ByteSlice { byte_start: url_start, byte_end: url_end },
            features: vec![LinkFeature { t: "app.bsky.richtext.facet#link", uri: recipe_url.to_string() }],
        }],
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    agent.repo().create_record(&user.did, "app.bsky.feed.post", &post).await?;
    Ok(())
}

async fn refresh_and_build_agent(
    user: &mut AuthenticatedUser,
    state: &AppState,
    session: &Session,
) -> anyhow::Result<Agent<DpopSession>> {
    let now = chrono::Utc::now();
    if now >= user.expires_at {
        if let Some(refresh_token) = user.refresh_token.clone() {
            let metadata = discovery::get_auth_server_metadata(&state.http_client, &user.pds_url).await?;
            let tokens = refresh_access_token(
                &state.http_client,
                &metadata.token_endpoint,
                &user.dpop_private_key_pem,
                &user.dpop_public_jwk,
                &state.client_id,
                &refresh_token,
            ).await?;
            user.access_token = tokens.access_token;
            user.refresh_token = tokens.refresh_token.or(Some(refresh_token));
            user.expires_at = chrono::Utc::now() + chrono::Duration::seconds(tokens.expires_in as i64);
            session.insert(USER_KEY, &*user).await
                .map_err(|e| anyhow::anyhow!("Failed to update session: {}", e))?;
        } else {
            return Err(anyhow::anyhow!("Token expired and no refresh token available"));
        }
    }
    let dpop_session = DpopSession::new(
        &user.did,
        &user.pds_url,
        &user.access_token,
        &user.dpop_private_key_pem,
        user.dpop_public_jwk.clone(),
    );
    Ok(Agent::with_http_client(dpop_session, state.http_client.clone()))
}

pub async fn delete_recipe(
    State(state): State<AppState>,
    session: Session,
    Path((handle, rkey)): Path<(String, String)>,
) -> Response {
    let mut user = match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };
    if user.handle != handle {
        return StatusCode::FORBIDDEN.into_response();
    }
    let result = async {
        let agent = refresh_and_build_agent(&mut user, &state, &session).await?;
        agent.repo().delete_record(&user.did, "eu.atchef.recipe", &rkey).await?;
        db::delete_recipe(&state.sqlite_pool, &rkey, &user.did).await?;
        Ok::<_, anyhow::Error>(())
    }.await;
    match result {
        Ok(_) => Redirect::to(&format!("/profile/{}", handle)).into_response(),
        Err(e) => {
            tracing::error!("Failed to delete recipe: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn edit_recipe_form(
    State(state): State<AppState>,
    session: Session,
    Path((handle, rkey)): Path<(String, String)>,
) -> Response {
    let user = match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };
    if user.handle != handle {
        return StatusCode::FORBIDDEN.into_response();
    }
    match db::get_recipe(&state.sqlite_pool, &handle, &rkey).await {
        Ok(Some(row)) => {
            let content = crate::views::edit_recipe_form_page(&handle, &rkey, &row.name, &row.description.unwrap_or_default(), row.portions.into(), row.prep_time.unwrap_or(0) as u64, row.cook_time.unwrap_or(0) as u64, &row.content, None);
            base_layout_with_user("Edit Recipe | AtChef", content, Some(&user.handle)).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn update_recipe(
    State(state): State<AppState>,
    session: Session,
    Path((handle, rkey)): Path<(String, String)>,
    multipart: Multipart,
) -> Response {
    let mut user = match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };
    if user.handle != handle {
        return StatusCode::FORBIDDEN.into_response();
    }
    let form = match parse_recipe_multipart(multipart).await {
        Ok(f) => f,
        Err(e) => {
            let content = crate::views::edit_recipe_form_page(&handle, &rkey, "", "", 4, 15, 30, "", Some(&format!("Invalid form data: {}", e)));
            return base_layout_with_user("Edit Recipe | AtChef", content, Some(&user.handle)).into_response();
        }
    };
    let result = async {
        let agent = refresh_and_build_agent(&mut user, &state, &session).await?;

        let portions = form.portions.max(1);
        let time = (form.prep_time + form.cook_time).max(1);
        let description = if form.description.trim().is_empty() { None } else { Some(form.description.trim().to_string()) };
        let prep_time = if form.prep_time > 0 { Some(form.prep_time) } else { None };
        let cook_time = if form.cook_time > 0 { Some(form.cook_time) } else { None };

        // Fetch existing record to preserve created_at and image
        let existing = db::get_recipe(&state.sqlite_pool, &handle, &rkey).await?
            .ok_or_else(|| anyhow::anyhow!("Recipe not found"))?;

        let image_blob = if let Some((image_data, mime_type)) = form.image {
            let blob_ref = agent.repo().upload_blob(image_data, &mime_type).await?;
            Some(blob_ref)
        } else {
            None
        };
        let converted_image = if let Some(ref blob) = image_blob {
            Some(convert_blob_ref(blob)?)
        } else {
            // Preserve existing image if no new one uploaded
            existing.image_cid.as_ref().and_then(|cid| {
                let mime = existing.image_mime_type.as_deref().unwrap_or("image/jpeg");
                use atrium_api::types::{BlobRef, TypedBlobRef, Blob};
                use std::str::FromStr;
                let cid_val = ipld_core::cid::Cid::from_str(cid).ok()?;
                Some(BlobRef::Typed(TypedBlobRef::Blob(Blob {
                    r#ref: atrium_api::types::CidLink(cid_val),
                    mime_type: mime.to_string(),
                    size: 0,
                })))
            })
        };

        let created_at = atrium_api::types::string::Datetime::new(
            existing.created_at.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        );

        let record = RecordData {
            name: form.name.clone(),
            description,
            portions: std::num::NonZeroU64::new(portions).unwrap(),
            time: std::num::NonZeroU64::new(time).unwrap(),
            prep_time,
            cook_time,
            content: form.content.clone(),
            image: converted_image,
            created_at,
        };

        agent.repo().put_record(&user.did, "eu.atchef.recipe", &rkey, &record).await?;

        let uri = format!("at://{}/eu.atchef.recipe/{}", user.did, rkey);
        let image_cid_ref = image_blob.as_ref().map(|b| b.cid());
        let image_mime_ref = image_blob.as_ref().map(|b| b.mime_type.as_str());
        let final_cid = image_cid_ref.as_deref().or(existing.image_cid.as_deref());
        let final_mime = image_mime_ref.or(existing.image_mime_type.as_deref());
        db::save_recipe(
            &state.sqlite_pool,
            &uri,
            &user.did,
            &user.handle,
            &rkey,
            &form.name,
            &form.content,
            portions as u32,
            time as u32,
            &existing.created_at.to_rfc3339(),
            record.description.as_deref(),
            record.prep_time.map(|v| v as u32),
            record.cook_time.map(|v| v as u32),
            final_cid,
            final_mime,
        ).await?;

        Ok::<_, anyhow::Error>(())
    }.await;
    match result {
        Ok(_) => Redirect::to(&format!("/profile/{}/recipe/{}", handle, rkey)).into_response(),
        Err(e) => {
            tracing::error!("Failed to update recipe: {}", e);
            let content = crate::views::edit_recipe_form_page(&handle, &rkey, &form.name, &form.description, form.portions, form.prep_time, form.cook_time, &form.content, Some(&format!("Failed to update recipe: {}", e)));
            base_layout_with_user("Edit Recipe | AtChef", content, Some(&user.handle)).into_response()
        }
    }
}

pub async fn chefs(State(state): State<AppState>, session: Session) -> Markup {
    let users = db::get_all_users(&state.sqlite_pool)
        .await
        .unwrap_or_default();
    
    let user = session
        .get::<AuthenticatedUser>(USER_KEY)
        .await
        .ok()
        .flatten();
    
    let user_handle = user.map(|u| u.handle);
    let content = crate::views::chefs_page(&users);
    base_layout_with_user("Chefs | AtChef", content, user_handle.as_deref())
}

pub async fn serve_blob(
    Path(cid): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // Try to get blob from cache first
    match state.blob_cache.get(&cid).await {
        Ok(Some(cached_blob)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                cached_blob.mime_type.parse().unwrap_or_else(|_| "application/octet-stream".parse().unwrap()),
            );
            headers.insert(
                header::CONTENT_LENGTH,
                cached_blob.size.to_string().parse().unwrap(),
            );
            headers.insert(
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".parse().unwrap(), // 1 year cache
            );
            headers.insert(
                header::ETAG,
                format!(r#""{}""#, cid).parse().unwrap(),
            );
            Ok((headers, cached_blob.data))
        }
        Ok(None) => {
            // Cache miss - need to fetch from PDS and cache it
            match fetch_and_cache_blob(&cid, &state).await {
                Ok(Some(blob_data)) => {
                    let mut headers = HeaderMap::new();
                    headers.insert(
                        header::CONTENT_TYPE,
                        blob_data.1.parse().unwrap_or_else(|_| "application/octet-stream".parse().unwrap()),
                    );
                    headers.insert(
                        header::CONTENT_LENGTH,
                        blob_data.0.len().to_string().parse().unwrap(),
                    );
                    headers.insert(
                        header::CACHE_CONTROL,
                        "public, max-age=31536000, immutable".parse().unwrap(),
                    );
                    headers.insert(
                        header::ETAG,
                        format!(r#""{}""#, cid).parse().unwrap(),
                    );
                    Ok((headers, blob_data.0))
                }
                Ok(None) => Err(StatusCode::NOT_FOUND),
                Err(e) => {
                    tracing::error!("Failed to fetch blob {}: {}", cid, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            tracing::error!("Blob cache error for {}: {}", cid, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn fetch_and_cache_blob(
    cid: &str,
    state: &AppState,
) -> anyhow::Result<Option<(Vec<u8>, String)>> {
    // First, try to find which author_did has a recipe with this image_cid
    let author_did: Option<String> = sqlx::query_scalar(
        "SELECT author_did FROM recipes WHERE image_cid = ? LIMIT 1"
    )
    .bind(cid)
    .fetch_optional(&state.sqlite_pool)
    .await?;

    let Some(did) = author_did else {
        tracing::debug!("No recipe found with image CID: {}", cid);
        return Ok(None);
    };

    // Extract PDS host from DID - AT Protocol DIDs typically follow format: did:plc:xxxxx
    // We need to resolve the DID to get the PDS URL
    let pds_url = resolve_pds_for_did(&did, &state.http_client).await?;
    
    // Fetch blob from the PDS
    let blob_url = format!("{}/xrpc/com.atproto.sync.getBlob?did={}&cid={}", 
        pds_url, 
        urlencoding::encode(&did), 
        urlencoding::encode(cid)
    );

    tracing::debug!("Fetching blob from: {}", blob_url);
    
    match state.http_client.get(&blob_url).send().await {
        Ok(response) if response.status().is_success() => {
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|ct| ct.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
                
            let data = response.bytes().await?;
            
            // Cache the blob for future requests
            if let Err(e) = state.blob_cache.store(cid, data.to_vec(), &content_type).await {
                tracing::warn!("Failed to cache blob {}: {}", cid, e);
            } else {
                tracing::debug!("Successfully cached blob: {}", cid);
            }
            
            Ok(Some((data.to_vec(), content_type)))
        }
        Ok(response) => {
            tracing::warn!("Failed to fetch blob {} from {}: HTTP {}", cid, pds_url, response.status());
            Ok(None)
        }
        Err(e) => {
            tracing::warn!("Error fetching blob {} from {}: {}", cid, pds_url, e);
            Ok(None)
        }
    }
}

/// Resolve a DID to find its PDS (Personal Data Server) URL
async fn resolve_pds_for_did(did: &str, client: &reqwest::Client) -> anyhow::Result<String> {
    // For did:plc: DIDs, resolve via the PLC directory
    if did.starts_with("did:plc:") {
        let plc_url = format!("https://plc.directory/{}", did);
        
        match client.get(&plc_url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(doc) = response.json::<serde_json::Value>().await {
                    // Look for PDS service endpoint in the DID document
                    if let Some(services) = doc.get("service").and_then(|s| s.as_array()) {
                        for service in services {
                            if let (Some(service_type), Some(endpoint)) = (
                                service.get("type").and_then(|t| t.as_str()),
                                service.get("serviceEndpoint").and_then(|e| e.as_str())
                            ) {
                                if service_type == "AtprotoPersonalDataServer" {
                                    return Ok(endpoint.to_string());
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    // For did:web: DIDs, extract hostname directly
    if did.starts_with("did:web:") {
        let hostname = did.strip_prefix("did:web:").unwrap_or("");
        return Ok(format!("https://{}", hostname));
    }
    
    // Fallback to common PDS instances
    tracing::debug!("Could not resolve PDS for DID {}, trying common instances", did);
    
    // Try bsky.social first as it's the most common
    Ok("https://bsky.social".to_string())
}
