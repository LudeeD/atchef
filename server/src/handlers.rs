use atproto_api::Agent;
use axum::{
    extract::{Path, Query, State},
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
    let recipes: Vec<Recipe> = db_recipes.iter().map(Recipe::from_db_row).collect();

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
    Path((handle, rkey)): Path<(String, String)>,
) -> Markup {
    let result = async {
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

        let recipe_detail = RecipeDetail {
            id: rkey.clone(),
            name: record.value.name,
            content: record.value.content,
            portions: record.value.portions as u32,
            time: record.value.time as u32,
            author_handle: handle.clone(),
            time_ago: time_ago(&record.value.created_at),
            comments: vec![],
        };

        let uri = format!("at://{}/eu.atchef.recipe/{}", did, rkey);
        let _ = db::save_recipe(
            &state.sqlite_pool,
            &rkey,
            &uri,
            &did,
            &handle,
            &rkey,
            &recipe_detail.name,
            &record.value.created_at,
        )
        .await;

        Ok(recipe_detail)
    }
    .await;

    match result {
        Ok(detail) => {
            let content = recipe_page(&detail);
            base_layout(&format!("{} | AtChef", detail.name), content)
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
            crate::models::Recipe {
                id: rkey,
                name: r.value.name,
                author_handle: handle.clone(),
                time_ago: time_ago(&r.value.created_at),
                comment_count: 0,
            }
        }).collect::<Vec<_>>();
        Ok((recipes, display_name, description, avatar_url))
    }
    .await;

    match result {
        Ok((recipes, display_name, description, avatar_url)) => {
            let content = crate::views::public_profile_page(
                &handle,
                &recipes,
                is_owner,
                display_name.as_deref(),
                description.as_deref(),
                avatar_url.as_deref(),
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

        let redirect_uri = format!("{}/oauth/callback", state.base_url);
        // For localhost development, use the loopback client ID format with scope
        let client_id = format!(
            "http://localhost?scope={}&redirect_uri={}",
            urlencoding::encode("atproto transition:generic"),
            urlencoding::encode(&redirect_uri)
        );
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256&scope=atproto%20transition:generic",
            as_metadata.authorization_endpoint,
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&oauth_state),
            urlencoding::encode(&pkce.challenge),
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
        // For localhost development, use the loopback client ID format with scope
        let client_id = format!(
            "http://localhost?scope={}&redirect_uri={}",
            urlencoding::encode("atproto transition:generic"),
            urlencoding::encode(&redirect_uri)
        );

        // Token exchange with DPoP nonce handling
        let tokens = exchange_token(
            &state.http_client,
            &pending.token_endpoint,
            &pending.dpop_private_key_pem,
            &pending.dpop_public_jwk,
            &client_id,
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
        client_id: state.base_url.clone(),
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

#[derive(Deserialize)]
pub struct RecipeForm {
    name: String,
    portions: u64,
    time: u64,
    content: String,
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

pub async fn create_recipe(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<RecipeForm>,
) -> Response {
    let mut user = match session.get::<AuthenticatedUser>(USER_KEY).await {
        Ok(Some(user)) => user,
        _ => return Redirect::to("/login").into_response(),
    };

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
                    &state.base_url,
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
        let recipe_name = form.name.clone();
        
        let record = RecordData {
            name: form.name,
            portions: std::num::NonZeroU64::new(form.portions.max(1)).unwrap(),
            time: std::num::NonZeroU64::new(form.time.max(1)).unwrap(),
            content: form.content,
            image: None,
            created_at: created_at.clone(),
        };

        let output = agent
            .repo()
            .create_record(&user.did, "eu.atchef.recipe", &record)
            .await?;

        Ok::<_, anyhow::Error>((output, created_at, recipe_name))
    }
    .await;

    match result {
        Ok((output, created_at, recipe_name)) => {
            let rkey = output.uri.split('/').last().unwrap_or("").to_string();
            let uri = output.uri.clone();

            if let Err(e) = db::save_recipe(
                &state.sqlite_pool,
                &rkey,
                &uri,
                &user.did,
                &user.handle,
                &rkey,
                &recipe_name,
                created_at.as_str(),
            ).await {
                tracing::error!("Failed to save recipe to database: {}", e);
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
