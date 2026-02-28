use reqwest::Client;

use crate::repo::RepoApi;
use crate::session::Session;

/// Main interface for ATProto API operations.
///
/// The `Agent` is parameterized by a `Session` implementation that handles
/// authentication. Use `BearerSession` for simple bearer token auth, or
/// implement your own `Session` for OAuth/DPoP.
///
/// # Example
///
/// ```ignore
/// use atproto_api::{Agent, BearerSession, Tid};
///
/// let session = BearerSession::new(
///     "did:plc:abc123",
///     "https://bsky.social",
///     "your-access-token",
/// );
///
/// let agent = Agent::new(session);
///
/// // Read a record
/// let profile = agent.repo().get_record::<Profile>(
///     agent.did(),
///     "app.bsky.actor.profile",
///     "self",
/// ).await?;
///
/// // Write a record
/// let rkey = Tid::now().to_string();
/// agent.repo().put_record(
///     agent.did(),
///     "my.app.record",
///     &rkey,
///     &MyRecord { value: 42 },
/// ).await?;
/// ```
pub struct Agent<S: Session> {
    session: S,
    http: Client,
}

impl<S: Session> Agent<S> {
    /// Create a new agent with the given session.
    pub fn new(session: S) -> Self {
        Self {
            session,
            http: Client::new(),
        }
    }

    /// Create a new agent with a custom HTTP client.
    pub fn with_http_client(session: S, http: Client) -> Self {
        Self { session, http }
    }

    /// Get the current user's DID.
    pub fn did(&self) -> &str {
        self.session.did()
    }

    /// Get the user's PDS URL.
    pub fn pds_url(&self) -> &str {
        self.session.pds_url()
    }

    /// Access repository operations (com.atproto.repo.*).
    pub fn repo(&self) -> RepoApi<'_, S> {
        RepoApi::new(&self.session, &self.http)
    }

    /// Get a reference to the underlying session.
    pub fn session(&self) -> &S {
        &self.session
    }
}
