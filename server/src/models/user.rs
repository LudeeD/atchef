use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BlobRef {
    #[serde(rename = "$type")]
    pub ref_type: String,
    #[serde(rename = "ref")]
    pub reference: serde_json::Value,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ProfileRecord {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<BlobRef>,
    pub banner: Option<BlobRef>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}
