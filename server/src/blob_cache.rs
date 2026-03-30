use std::sync::Arc;
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct CachedBlob {
    pub cid: String,
    pub data: Vec<u8>,
    pub mime_type: String,
    pub size: u64,
}

pub struct BlobCacheService {
    db: Arc<SqlitePool>,
    max_size_bytes: u64,
}

impl BlobCacheService {
    pub fn new(db: Arc<SqlitePool>, max_size_mb: u64) -> Self {
        Self {
            db,
            max_size_bytes: max_size_mb * 1024 * 1024, // Convert MB to bytes
        }
    }

    /// Get a blob from cache, updating access time and count
    pub async fn get(&self, cid: &str) -> anyhow::Result<Option<CachedBlob>> {
        // First update access statistics
        sqlx::query(
            r#"
            UPDATE blob_cache 
            SET last_accessed = CURRENT_TIMESTAMP, access_count = access_count + 1 
            WHERE cid = ?
            "#,
        )
        .bind(cid)
        .execute(&*self.db)
        .await?;

        // Then fetch the blob
        let row: Option<(String, Vec<u8>, String, i64)> = sqlx::query_as(
            "SELECT cid, data, mime_type, size FROM blob_cache WHERE cid = ?",
        )
        .bind(cid)
        .fetch_optional(&*self.db)
        .await?;

        Ok(row.map(|(cid, data, mime_type, size)| CachedBlob {
            cid,
            data,
            mime_type,
            size: size as u64,
        }))
    }

    /// Store a blob in cache, evicting old entries if necessary
    pub async fn store(&self, cid: &str, data: Vec<u8>, mime_type: &str) -> anyhow::Result<()> {
        let size = data.len() as u64;
        
        // Check if we need to make space
        let current_size = self.get_total_cache_size().await?;
        if current_size + size > self.max_size_bytes {
            let target_free = size.max(self.max_size_bytes / 10); // Free at least 10% or enough for new blob
            self.evict_lru(target_free).await?;
        }

        // Insert or replace the blob
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO blob_cache (cid, data, mime_type, size, created_at, last_accessed, access_count)
            VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 1)
            "#,
        )
        .bind(cid)
        .bind(&data)
        .bind(mime_type)
        .bind(size as i64)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// Evict least recently used blobs to free up space
    pub async fn evict_lru(&self, target_free_bytes: u64) -> anyhow::Result<u64> {
        let mut freed_bytes = 0u64;

        // Get LRU blobs ordered by last_accessed
        let lru_blobs: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT cid, size 
            FROM blob_cache 
            ORDER BY last_accessed ASC, access_count ASC
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        for (cid, size) in lru_blobs {
            if freed_bytes >= target_free_bytes {
                break;
            }

            sqlx::query("DELETE FROM blob_cache WHERE cid = ?")
                .bind(&cid)
                .execute(&*self.db)
                .await?;

            freed_bytes += size as u64;
        }

        Ok(freed_bytes)
    }

    /// Get total size of cached blobs
    pub async fn get_total_cache_size(&self) -> anyhow::Result<u64> {
        let size: Option<i64> = sqlx::query_scalar("SELECT SUM(size) FROM blob_cache")
            .fetch_optional(&*self.db)
            .await?;
        Ok(size.unwrap_or(0) as u64)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> anyhow::Result<CacheStats> {
        let (total_blobs, total_size, avg_access_count): (i64, i64, f64) = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total_blobs,
                COALESCE(SUM(size), 0) as total_size,
                COALESCE(AVG(access_count), 0.0) as avg_access_count
            FROM blob_cache
            "#,
        )
        .fetch_one(&*self.db)
        .await?;

        Ok(CacheStats {
            total_blobs: total_blobs as u64,
            total_size_bytes: total_size as u64,
            max_size_bytes: self.max_size_bytes,
            utilization_percent: (total_size as f64 / self.max_size_bytes as f64 * 100.0).min(100.0),
            avg_access_count: avg_access_count,
        })
    }

    /// Warm cache with a list of CIDs by pre-fetching them
    pub async fn warm_cache(&self, cids: Vec<String>, fetch_fn: impl Fn(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Option<(Vec<u8>, String)>>> + Send>>) -> anyhow::Result<u64> {
        let mut cached_count = 0u64;

        for cid in cids {
            // Skip if already cached
            if self.get(&cid).await?.is_some() {
                continue;
            }

            // Attempt to fetch and cache
            if let Ok(Some((data, mime_type))) = fetch_fn(&cid).await {
                if self.store(&cid, data, &mime_type).await.is_ok() {
                    cached_count += 1;
                }
            }
        }

        Ok(cached_count)
    }

    /// Clean up old or unused blobs (background maintenance)
    pub async fn cleanup_stale_blobs(&self, max_age_days: u32) -> anyhow::Result<u64> {
        let cutoff_timestamp = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        
        let deleted_count: i64 = sqlx::query_scalar(
            r#"
            DELETE FROM blob_cache 
            WHERE last_accessed < ? AND access_count = 1
            RETURNING COUNT(*)
            "#,
        )
        .bind(cutoff_timestamp.to_rfc3339())
        .fetch_one(&*self.db)
        .await?;

        Ok(deleted_count as u64)
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_blobs: u64,
    pub total_size_bytes: u64,
    pub max_size_bytes: u64,
    pub utilization_percent: f64,
    pub avg_access_count: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Blob Cache: {} blobs, {:.1}MB/{:.1}MB ({:.1}% full), avg access: {:.1}",
            self.total_blobs,
            self.total_size_bytes as f64 / 1024.0 / 1024.0,
            self.max_size_bytes as f64 / 1024.0 / 1024.0,
            self.utilization_percent,
            self.avg_access_count
        )
    }
}