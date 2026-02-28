use serde::{Deserialize, Serialize};

use crate::types::BlobRef;

/// Response from com.atproto.repo.getRecord
#[derive(Debug, Clone, Deserialize)]
pub struct GetRecordOutput<T> {
    pub uri: String,
    pub cid: Option<String>,
    pub value: T,
}

/// Input for com.atproto.repo.putRecord
#[derive(Debug, Clone, Serialize)]
pub struct PutRecordInput<'a, T> {
    pub repo: &'a str,
    pub collection: &'a str,
    pub rkey: &'a str,
    pub record: &'a T,
    #[serde(rename = "swapRecord", skip_serializing_if = "Option::is_none")]
    pub swap_record: Option<&'a str>,
    #[serde(rename = "swapCommit", skip_serializing_if = "Option::is_none")]
    pub swap_commit: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<bool>,
}

/// Response from com.atproto.repo.putRecord
#[derive(Debug, Clone, Deserialize)]
pub struct PutRecordOutput {
    pub uri: String,
    pub cid: String,
}

/// Input for com.atproto.repo.createRecord
#[derive(Debug, Clone, Serialize)]
pub struct CreateRecordInput<'a, T> {
    pub repo: &'a str,
    pub collection: &'a str,
    pub record: &'a T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rkey: Option<&'a str>,
    #[serde(rename = "swapCommit", skip_serializing_if = "Option::is_none")]
    pub swap_commit: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<bool>,
}

/// Response from com.atproto.repo.createRecord
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecordOutput {
    pub uri: String,
    pub cid: String,
}

/// Input for com.atproto.repo.deleteRecord
#[derive(Debug, Clone, Serialize)]
pub struct DeleteRecordInput<'a> {
    pub repo: &'a str,
    pub collection: &'a str,
    pub rkey: &'a str,
    #[serde(rename = "swapRecord", skip_serializing_if = "Option::is_none")]
    pub swap_record: Option<&'a str>,
    #[serde(rename = "swapCommit", skip_serializing_if = "Option::is_none")]
    pub swap_commit: Option<&'a str>,
}

/// Response from com.atproto.repo.listRecords
#[derive(Debug, Clone, Deserialize)]
pub struct ListRecordsOutput<T> {
    pub records: Vec<ListRecordsRecord<T>>,
    pub cursor: Option<String>,
}

/// A single record in listRecords response
#[derive(Debug, Clone, Deserialize)]
pub struct ListRecordsRecord<T> {
    pub uri: String,
    pub cid: String,
    pub value: T,
}

/// Response from com.atproto.repo.uploadBlob
#[derive(Debug, Clone, Deserialize)]
pub struct UploadBlobOutput {
    pub blob: BlobRef,
}
