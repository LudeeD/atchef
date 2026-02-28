use std::fmt;
use std::str::FromStr;

use crate::Error;

/// An AT Protocol URI.
/// Format: at://did:plc:abc123/collection/rkey
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AtUri {
    authority: String, // DID or handle
    collection: Option<String>,
    rkey: Option<String>,
}

impl AtUri {
    /// Create a new AT URI from components.
    pub fn new(
        authority: impl Into<String>,
        collection: Option<impl Into<String>>,
        rkey: Option<impl Into<String>>,
    ) -> Self {
        Self {
            authority: authority.into(),
            collection: collection.map(Into::into),
            rkey: rkey.map(Into::into),
        }
    }

    /// Create an AT URI for a specific record.
    pub fn record(repo: impl Into<String>, collection: impl Into<String>, rkey: impl Into<String>) -> Self {
        Self {
            authority: repo.into(),
            collection: Some(collection.into()),
            rkey: Some(rkey.into()),
        }
    }

    /// Create an AT URI for a collection.
    pub fn for_collection(repo: impl Into<String>, collection: impl Into<String>) -> Self {
        Self {
            authority: repo.into(),
            collection: Some(collection.into()),
            rkey: None,
        }
    }

    /// Create an AT URI for a repo.
    pub fn repo(authority: impl Into<String>) -> Self {
        Self {
            authority: authority.into(),
            collection: None,
            rkey: None,
        }
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }

    pub fn collection(&self) -> Option<&str> {
        self.collection.as_deref()
    }

    pub fn rkey(&self) -> Option<&str> {
        self.rkey.as_deref()
    }
}

impl fmt::Display for AtUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at://{}", self.authority)?;
        if let Some(ref collection) = self.collection {
            write!(f, "/{}", collection)?;
            if let Some(ref rkey) = self.rkey {
                write!(f, "/{}", rkey)?;
            }
        }
        Ok(())
    }
}

impl FromStr for AtUri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s
            .strip_prefix("at://")
            .ok_or_else(|| Error::InvalidAtUri("must start with 'at://'".into()))?;

        let mut parts = s.splitn(3, '/');

        let authority = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::InvalidAtUri("missing authority".into()))?
            .to_string();

        let collection = parts.next().filter(|s| !s.is_empty()).map(String::from);
        let rkey = parts.next().filter(|s| !s.is_empty()).map(String::from);

        // Can't have rkey without collection
        if rkey.is_some() && collection.is_none() {
            return Err(Error::InvalidAtUri("rkey without collection".into()));
        }

        Ok(Self {
            authority,
            collection,
            rkey,
        })
    }
}

impl serde::Serialize for AtUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for AtUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AtUri::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_uri() {
        let uri: AtUri = "at://did:plc:abc123/app.bsky.feed.post/3k2f5v".parse().unwrap();
        assert_eq!(uri.authority(), "did:plc:abc123");
        assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(uri.rkey(), Some("3k2f5v"));
    }

    #[test]
    fn test_parse_collection_uri() {
        let uri: AtUri = "at://alice.bsky.social/app.bsky.feed.post".parse().unwrap();
        assert_eq!(uri.authority(), "alice.bsky.social");
        assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(uri.rkey(), None);
    }

    #[test]
    fn test_parse_repo_uri() {
        let uri: AtUri = "at://did:plc:abc123".parse().unwrap();
        assert_eq!(uri.authority(), "did:plc:abc123");
        assert_eq!(uri.collection(), None);
        assert_eq!(uri.rkey(), None);
    }

    #[test]
    fn test_roundtrip() {
        let original = "at://did:plc:abc123/app.bsky.feed.post/3k2f5v";
        let uri: AtUri = original.parse().unwrap();
        assert_eq!(uri.to_string(), original);
    }

    #[test]
    fn test_invalid_uri() {
        assert!(AtUri::from_str("https://example.com").is_err());
        assert!(AtUri::from_str("at://").is_err());
    }
}
