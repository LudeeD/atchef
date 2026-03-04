use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio_tungstenite::connect_async;

use crate::{db, oauth::discovery};

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
}

pub async fn run(client: reqwest::Client, pool: SqlitePool) {
    loop {
        if let Err(e) = connect_and_consume(&client, &pool).await {
            tracing::error!("jetstream sync error: {e}, reconnecting in 5s...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn connect_and_consume(client: &reqwest::Client, pool: &SqlitePool) -> anyhow::Result<()> {
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
                )
                .await
                {
                    tracing::warn!("failed to save recipe {}: {e}", uri);
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
