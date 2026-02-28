use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::types::*;
use crate::session::Session;
use crate::types::BlobRef;
use crate::xrpc::XrpcClient;
use crate::Error;

/// Repository operations (com.atproto.repo.*)
pub struct RepoApi<'a, S: Session> {
    session: &'a S,
    http: &'a Client,
}

impl<'a, S: Session> RepoApi<'a, S> {
    pub(crate) fn new(session: &'a S, http: &'a Client) -> Self {
        Self { session, http }
    }

    /// Get a single record.
    ///
    /// # Arguments
    /// * `repo` - The DID of the repo
    /// * `collection` - The NSID of the collection (e.g., "app.bsky.feed.post")
    /// * `rkey` - The record key
    pub async fn get_record<T: DeserializeOwned>(
        &self,
        repo: &str,
        collection: &str,
        rkey: &str,
    ) -> Result<GetRecordOutput<T>, Error> {
        let client = XrpcClient::new(self.session, self.http);
        client
            .get(
                "com.atproto.repo.getRecord",
                &[("repo", repo), ("collection", collection), ("rkey", rkey)],
            )
            .await
    }

    /// Create or update a record at a specific rkey.
    ///
    /// # Arguments
    /// * `repo` - The DID of the repo
    /// * `collection` - The NSID of the collection
    /// * `rkey` - The record key
    /// * `record` - The record data
    pub async fn put_record<T: Serialize>(
        &self,
        repo: &str,
        collection: &str,
        rkey: &str,
        record: &T,
    ) -> Result<PutRecordOutput, Error> {
        let client = XrpcClient::new(self.session, self.http);
        let input = PutRecordInput {
            repo,
            collection,
            rkey,
            record,
            swap_record: None,
            swap_commit: None,
            validate: None,
        };
        client.post("com.atproto.repo.putRecord", &input).await
    }

    /// Create or update a record with additional options.
    pub async fn put_record_with_options<T: Serialize>(
        &self,
        repo: &str,
        collection: &str,
        rkey: &str,
        record: &T,
        swap_record: Option<&str>,
        swap_commit: Option<&str>,
        validate: Option<bool>,
    ) -> Result<PutRecordOutput, Error> {
        let client = XrpcClient::new(self.session, self.http);
        let input = PutRecordInput {
            repo,
            collection,
            rkey,
            record,
            swap_record,
            swap_commit,
            validate,
        };
        client.post("com.atproto.repo.putRecord", &input).await
    }

    /// Create a new record.
    ///
    /// # Arguments
    /// * `repo` - The DID of the repo
    /// * `collection` - The NSID of the collection
    /// * `record` - The record data
    pub async fn create_record<T: Serialize>(
        &self,
        repo: &str,
        collection: &str,
        record: &T,
    ) -> Result<CreateRecordOutput, Error> {
        let client = XrpcClient::new(self.session, self.http);
        let input = CreateRecordInput {
            repo,
            collection,
            record,
            rkey: None,
            swap_commit: None,
            validate: None,
        };
        client.post("com.atproto.repo.createRecord", &input).await
    }

    /// Create a new record with a specific rkey.
    pub async fn create_record_with_rkey<T: Serialize>(
        &self,
        repo: &str,
        collection: &str,
        rkey: &str,
        record: &T,
    ) -> Result<CreateRecordOutput, Error> {
        let client = XrpcClient::new(self.session, self.http);
        let input = CreateRecordInput {
            repo,
            collection,
            record,
            rkey: Some(rkey),
            swap_commit: None,
            validate: None,
        };
        client.post("com.atproto.repo.createRecord", &input).await
    }

    /// Delete a record.
    ///
    /// # Arguments
    /// * `repo` - The DID of the repo
    /// * `collection` - The NSID of the collection
    /// * `rkey` - The record key
    pub async fn delete_record(
        &self,
        repo: &str,
        collection: &str,
        rkey: &str,
    ) -> Result<(), Error> {
        let client = XrpcClient::new(self.session, self.http);
        let input = DeleteRecordInput {
            repo,
            collection,
            rkey,
            swap_record: None,
            swap_commit: None,
        };
        client
            .post_no_response("com.atproto.repo.deleteRecord", &input)
            .await
    }

    /// List records in a collection.
    ///
    /// # Arguments
    /// * `repo` - The DID of the repo
    /// * `collection` - The NSID of the collection
    pub async fn list_records<T: DeserializeOwned>(
        &self,
        repo: &str,
        collection: &str,
    ) -> Result<ListRecordsOutput<T>, Error> {
        let client = XrpcClient::new(self.session, self.http);
        client
            .get(
                "com.atproto.repo.listRecords",
                &[("repo", repo), ("collection", collection)],
            )
            .await
    }

    /// List records with pagination.
    ///
    /// # Arguments
    /// * `repo` - The DID of the repo
    /// * `collection` - The NSID of the collection
    /// * `limit` - Maximum number of records to return
    /// * `cursor` - Pagination cursor from previous response
    /// * `reverse` - If true, return oldest records first
    pub async fn list_records_with_options<T: DeserializeOwned>(
        &self,
        repo: &str,
        collection: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
        reverse: Option<bool>,
    ) -> Result<ListRecordsOutput<T>, Error> {
        let client = XrpcClient::new(self.session, self.http);

        let mut params: Vec<(&str, &str)> = vec![("repo", repo), ("collection", collection)];

        let limit_str;
        if let Some(l) = limit {
            limit_str = l.to_string();
            params.push(("limit", &limit_str));
        }

        if let Some(c) = cursor {
            params.push(("cursor", c));
        }

        let reverse_str;
        if let Some(r) = reverse {
            reverse_str = r.to_string();
            params.push(("reverse", &reverse_str));
        }

        client.get("com.atproto.repo.listRecords", &params).await
    }

    /// Upload a blob (image, file, etc.).
    ///
    /// # Arguments
    /// * `data` - The blob data
    /// * `mime_type` - The MIME type (e.g., "image/jpeg")
    ///
    /// # Returns
    /// A `BlobRef` that can be embedded in records.
    pub async fn upload_blob(&self, data: Vec<u8>, mime_type: &str) -> Result<BlobRef, Error> {
        let client = XrpcClient::new(self.session, self.http);
        let output: UploadBlobOutput = client
            .post_bytes("com.atproto.repo.uploadBlob", data, mime_type)
            .await?;
        Ok(output.blob)
    }
}
