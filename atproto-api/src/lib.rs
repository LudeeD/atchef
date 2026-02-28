//! ATProto API Client
//!
//! A Rust crate for reading and writing records to ATProto repositories.
//!
//! This crate does not bundle OAuth - it accepts credentials via a `Session` trait.
//! Implement `Session` in your OAuth crate, or use `BearerSession` for simple
//! bearer token authentication (app passwords, testing).
//!
//! # Example
//!
//! ```ignore
//! use atproto_api::{Agent, BearerSession, Tid};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct StatusRecord {
//!     status: String,
//!     #[serde(rename = "createdAt")]
//!     created_at: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a session (your OAuth implementation would provide this)
//!     let session = BearerSession::new(
//!         "did:plc:abc123",
//!         "https://bsky.social",
//!         "your-access-token",
//!     );
//!
//!     // Create agent
//!     let agent = Agent::new(session);
//!
//!     // Write a record
//!     let rkey = Tid::now().to_string();
//!     agent.repo().put_record(
//!         agent.did(),
//!         "xyz.statusphere.status",
//!         &rkey,
//!         &StatusRecord {
//!             status: "👍".into(),
//!             created_at: chrono::Utc::now().to_rfc3339(),
//!         },
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

mod agent;
mod error;
pub mod repo;
mod session;
pub mod types;
mod xrpc;

pub use agent::Agent;
pub use error::Error;
pub use session::{BearerSession, Session};
pub use types::{AtUri, BlobRef, Did, Handle, Tid};

// Re-export repo types at top level for convenience
pub use repo::{
    CreateRecordOutput, GetRecordOutput, ListRecordsOutput, ListRecordsRecord, PutRecordOutput,
};
