use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

/// A Decentralized Identifier (DID).
/// Format: did:method:identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Did(String);

impl Did {
    pub fn new(did: impl Into<String>) -> Result<Self, Error> {
        let did = did.into();
        if !did.starts_with("did:") {
            return Err(Error::InvalidDid(did));
        }
        let parts: Vec<&str> = did.splitn(3, ':').collect();
        if parts.len() < 3 || parts[1].is_empty() || parts[2].is_empty() {
            return Err(Error::InvalidDid(did));
        }
        Ok(Self(did))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Did {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Did::new(s)
    }
}

impl AsRef<str> for Did {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// An ATProto handle (domain-based identifier).
/// Format: user.bsky.social
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Handle(String);

impl Handle {
    pub fn new(handle: impl Into<String>) -> Result<Self, Error> {
        let handle = handle.into();
        // Basic validation: must have at least one dot and valid characters
        if !handle.contains('.') || handle.starts_with('.') || handle.ends_with('.') {
            return Err(Error::InvalidHandle(handle));
        }
        // Check for valid characters (alphanumeric, dots, hyphens)
        for part in handle.split('.') {
            if part.is_empty() {
                return Err(Error::InvalidHandle(handle));
            }
            if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(Error::InvalidHandle(handle));
            }
        }
        Ok(Self(handle))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Handle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Handle::new(s)
    }
}

impl AsRef<str> for Handle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_did() {
        assert!(Did::new("did:plc:abc123").is_ok());
        assert!(Did::new("did:web:example.com").is_ok());
    }

    #[test]
    fn test_invalid_did() {
        assert!(Did::new("notadid").is_err());
        assert!(Did::new("did:").is_err());
        assert!(Did::new("did:plc:").is_err());
    }

    #[test]
    fn test_valid_handle() {
        assert!(Handle::new("alice.bsky.social").is_ok());
        assert!(Handle::new("test.example.com").is_ok());
    }

    #[test]
    fn test_invalid_handle() {
        assert!(Handle::new("nodomainpart").is_err());
        assert!(Handle::new(".invalid").is_err());
        assert!(Handle::new("invalid.").is_err());
    }
}
