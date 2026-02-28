use serde::{Deserialize, Serialize};

/// Reference to an uploaded blob (image, file, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRef {
    #[serde(rename = "$type")]
    pub type_marker: String,
    #[serde(rename = "ref")]
    pub cid: CidLink,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: u64,
}

impl BlobRef {
    pub fn new(cid: impl Into<String>, mime_type: impl Into<String>, size: u64) -> Self {
        Self {
            type_marker: "blob".to_string(),
            cid: CidLink { link: cid.into() },
            mime_type: mime_type.into(),
            size,
        }
    }

    pub fn cid(&self) -> &str {
        &self.cid.link
    }
}

/// CID link wrapper for blob references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CidLink {
    #[serde(rename = "$link")]
    pub link: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_ref_serialization() {
        let blob = BlobRef::new(
            "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku",
            "image/jpeg",
            12345,
        );

        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("\"$type\":\"blob\""));
        assert!(json.contains("\"$link\""));
        assert!(json.contains("\"mimeType\":\"image/jpeg\""));

        let parsed: BlobRef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, blob);
    }
}
