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
use crate::models::{get_mock_recipe_detail, get_mock_recipes, ProfileRecord};
use crate::oauth::{discovery, dpop, pkce, AuthenticatedUser, DpopSession, PendingAuth};
use crate::views::{base_layout, base_layout_with_user, login_page, profile_page, recipe_form_page, recipe_list, recipe_page};
use crate::AppState;

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

pub async fn home(session: Session) -> Markup {
    let recipes = get_mock_recipes();
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
        Ok(Some(user)) => {
            let content = profile_page(&user);
            base_layout_with_user("Profile | AtChef", content, Some(&user.handle)).into_response()
        }
        _ => Redirect::to("/login").into_response(),
    }
}

pub async fn recipe(Path(id): Path<String>) -> Markup {
    match get_mock_recipe_detail(&id) {
        Some(recipe) => {
            let content = recipe_page(&recipe);
            base_layout(&format!("{} | AtChef", recipe.name), content)
        }
        None => base_layout(
            "Not Found | AtChef",
            maud::html! {
                h1 { "Recipe not found" }
                p { "The recipe you're looking for doesn't exist." }
                p { a href="/" { "Back to home" } }
            },
        ),
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

        let record = RecordData {
            name: form.name,
            portions: std::num::NonZeroU64::new(form.portions.max(1)).unwrap(),
            time: std::num::NonZeroU64::new(form.time.max(1)).unwrap(),
            content: form.content,
            image: None,
            created_at: atrium_api::types::string::Datetime::now(),
        };

        let output = agent
            .repo()
            .create_record(&user.did, "eu.atchef.recipe", &record)
            .await?;

        Ok::<_, anyhow::Error>(output)
    }
    .await;

    match result {
        Ok(output) => {
            let uri = output.uri;
            let rkey = uri.split('/').last().unwrap_or(&uri);
            Redirect::to(&format!("/recipe/{}", rkey)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create recipe: {}", e);
            let content = recipe_form_page(Some(&format!("Failed to create recipe: {}", e)));
            base_layout_with_user("New Recipe | AtChef", content, Some(&user.handle)).into_response()
        }
    }
}
