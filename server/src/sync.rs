use std::time::Duration;
use std::sync::Arc;

use futures_util::StreamExt;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio_tungstenite::connect_async;

use crate::{db, oauth::discovery, blob_cache::BlobCacheService};
use urlencoding;

#[derive(Deserialize)]
struct JetstreamEvent {
    did: String,
    time_us: i64,
    kind: String,
    commit: Option<JetstreamCommit>,
}

#[derive(Deserialize)]
struct JetstreamCommit {
    operation: String,
    #[allow(dead_code)]
    collection: String,
    rkey: String,
    record: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RecipeRecord {
    name: String,
    content: String,
    portions: Option<u64>,
    time: Option<u64>,
    #[serde(rename = "createdAt")]
    created_at: String,
    description: Option<String>,
    #[serde(rename = "prepTime")]
    prep_time: Option<u64>,
    #[serde(rename = "cookTime")]
    cook_time: Option<u64>,
    image: Option<serde_json::Value>,
}

pub async fn run(client: reqwest::Client, pool: SqlitePool, blob_cache: Arc<BlobCacheService>) {
    loop {
        if let Err(e) = connect_and_consume(&client, &pool, &blob_cache).await {
            tracing::error!("jetstream sync error: {e}, reconnecting in 5s...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn connect_and_consume(client: &reqwest::Client, pool: &SqlitePool, blob_cache: &Arc<BlobCacheService>) -> anyhow::Result<()> {
    let cursor = db::get_cursor(pool).await?;
    let url = match cursor {
        Some(c) => format!(
            "wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=eu.atchef.recipe&cursor={}",
            c
        ),
        None => "wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=eu.atchef.recipe".to_string(),
    };

    tracing::info!("connecting to jetstream (cursor: {:?})", cursor);
    let (ws_stream, _) = connect_async(&url).await?;
    tracing::info!("connected to jetstream");

    let (_, mut read) = ws_stream.split();
    let mut event_count: u64 = 0;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            _ => continue,
        };

        let event: JetstreamEvent = match serde_json::from_str(&text) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("failed to parse jetstream event: {e}");
                continue;
            }
        };

        if event.kind != "commit" {
            event_count += 1;
            if event_count % 100 == 0 {
                let _ = db::save_cursor(pool, event.time_us).await;
            }
            continue;
        }

        let commit = match event.commit {
            Some(c) => c,
            None => continue,
        };

        match commit.operation.as_str() {
            "create" | "update" => {
                let record_val = match commit.record {
                    Some(v) => v,
                    None => continue,
                };
                let record: RecipeRecord = match serde_json::from_value(record_val) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("failed to parse recipe record: {e}");
                        continue;
                    }
                };
                let handle = match discovery::resolve_did_to_handle(client, &event.did).await {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::warn!("failed to resolve DID {}: {e}", event.did);
                        continue;
                    }
                };
                let uri = format!("at://{}/eu.atchef.recipe/{}", event.did, commit.rkey);
                let image_cid = record.image.as_ref()
                    .and_then(|img| img.get("cid"))
                    .and_then(|cid| cid.as_str())
                    .map(String::from);
                let image_mime_type = record.image.as_ref()
                    .and_then(|img| img.get("mimeType"))
                    .and_then(|mime| mime.as_str())
                    .map(String::from);

                if let Err(e) = db::save_recipe(
                    pool,
                    &uri,
                    &event.did,
                    &handle,
                    &commit.rkey,
                    &record.name,
                    &record.content,
                    record.portions.unwrap_or(0) as u32,
                    record.time.unwrap_or(0) as u32,
                    &record.created_at,
                    record.description.as_deref(),
                    record.prep_time.map(|v| v as u32),
                    record.cook_time.map(|v| v as u32),
                    image_cid.as_deref(),
                    image_mime_type.as_deref(),
                )
                .await
                {
                    tracing::warn!("failed to save recipe {}: {e}", uri);
                } else {
                    // Recipe saved successfully - warm cache with image if present
                    if let Some(cid) = image_cid {
                        let cid = cid.clone();
                        let blob_cache = blob_cache.clone();
                        let client = client.clone();
                        let event_did = event.did.clone();
                        let mime_type = image_mime_type.unwrap_or_else(|| "image/jpeg".to_string());
                        
                        tokio::spawn(async move {
                            // Check if blob is already cached
                            if blob_cache.get(&cid).await.unwrap_or(None).is_none() {
                                // Try to fetch the blob and cache it
                                let pds_url = format!("https://{}", event_did); // Simplified PDS URL construction
                                let blob_url = format!("{}/xrpc/com.atproto.sync.getBlob?did={}&cid={}", 
                                    pds_url, 
                                    urlencoding::encode(&event_did), 
                                    urlencoding::encode(&cid));
                                
                                match client.get(&blob_url).send().await {
                                    Ok(response) if response.status().is_success() => {
                                        if let Ok(data) = response.bytes().await {
                                            if let Err(e) = blob_cache.store(&cid, data.to_vec(), &mime_type).await {
                                                tracing::warn!("Failed to cache blob {}: {}", cid, e);
                                            } else {
                                                tracing::debug!("Cached recipe image blob: {}", cid);
                                            }
                                        }
                                    }
                                    Ok(response) => {
                                        tracing::debug!("Failed to fetch blob {} from {}: {}", cid, pds_url, response.status());
                                    }
                                    Err(e) => {
                                        tracing::debug!("Error fetching blob {} from {}: {}", cid, pds_url, e);
                                    }
                                }
                            }
                        });
                    }
                }
            }
            "delete" => {
                if let Err(e) = db::delete_recipe(pool, &commit.rkey, &event.did).await {
                    tracing::warn!("failed to delete recipe {}/{}: {e}", event.did, commit.rkey);
                }
            }
            _ => {}
        }

        event_count += 1;
        if event_count % 100 == 0 {
            let _ = db::save_cursor(pool, event.time_us).await;
        }
    }

    Ok(())
}
