# atproto-api

A Rust crate for reading and writing records to ATProto repositories. Does not bundle OAuth - accepts credentials via a `Session` trait.

## Structure

```
src/
├── lib.rs              # Main exports
├── error.rs            # Error types
├── session.rs          # Session trait + BearerSession
├── agent.rs            # Main Agent struct
├── types/
│   ├── did.rs          # Did, Handle newtypes
│   ├── tid.rs          # TID generation
│   ├── at_uri.rs       # AT URI parsing
│   └── blob.rs         # BlobRef type
├── xrpc/
│   └── client.rs       # HTTP client with Session auth
└── repo/
    ├── api.rs          # getRecord, putRecord, etc.
    └── types.rs        # Request/response types
```

## Exports

- `Agent<S: Session>` - Main interface parameterized by session type
- `Session` trait - Implement this for OAuth/DPoP auth
- `BearerSession` - Simple bearer token auth for testing/app passwords
- `Tid` - Timestamp-based record key generation
- `AtUri`, `Did`, `Handle`, `BlobRef` - ATProto types
- `RepoApi` - Repository operations (get/put/create/delete records, upload blobs)

## Usage

```rust
use atproto_api::{Agent, BearerSession, Tid};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct StatusRecord {
    status: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a session (your OAuth implementation provides this)
    let session = BearerSession::new(
        "did:plc:abc123",
        "https://bsky.social",
        "your-access-token",
    );

    let agent = Agent::new(session);

    // Write a record
    let rkey = Tid::now().to_string();
    agent.repo().put_record(
        agent.did(),
        "xyz.statusphere.status",
        &rkey,
        &StatusRecord {
            status: "👍".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    ).await?;

    // Read a record
    let record = agent.repo().get_record::<StatusRecord>(
        agent.did(),
        "xyz.statusphere.status",
        &rkey,
    ).await?;

    Ok(())
}
```

## Implementing Custom Session

For OAuth/DPoP authentication, implement the `Session` trait:

```rust
use async_trait::async_trait;
use atproto_api::{Session, Error};

struct MyOAuthSession {
    did: String,
    pds_url: String,
    // ... OAuth state
}

#[async_trait]
impl Session for MyOAuthSession {
    fn did(&self) -> &str { &self.did }
    fn pds_url(&self) -> &str { &self.pds_url }

    async fn get_auth_headers(
        &self,
        method: &str,
        url: &str,
    ) -> Result<Vec<(String, String)>, Error> {
        // Generate DPoP proof and return headers
        Ok(vec![
            ("Authorization".into(), format!("DPoP {}", self.access_token())),
            ("DPoP".into(), self.create_dpop_proof(method, url)?),
        ])
    }
}
```

## Adding to Your Project

```toml
[dependencies]
atproto-api = { path = "../atproto-api" }
```
