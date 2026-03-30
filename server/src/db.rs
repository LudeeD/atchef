use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    // Migrate old schema (had synthetic 'id' PK) → drop and recreate with composite PK
    let has_old_schema = sqlx::query("SELECT id FROM recipes LIMIT 0")
        .execute(pool)
        .await
        .is_ok();
    if has_old_schema {
        sqlx::query("DROP TABLE recipes").execute(pool).await?;
    }

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recipes (
            author_did TEXT NOT NULL,
            rkey TEXT NOT NULL,
            uri TEXT NOT NULL,
            author_handle TEXT NOT NULL,
            name TEXT NOT NULL,
            content TEXT,
            portions INTEGER,
            time INTEGER,
            created_at TEXT NOT NULL,
            description TEXT,
            prep_time INTEGER,
            cook_time INTEGER,
            image_cid TEXT,
            image_mime_type TEXT,
            PRIMARY KEY (author_did, rkey)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            did TEXT PRIMARY KEY,
            handle TEXT NOT NULL,
            first_login_at TEXT NOT NULL,
            last_login_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sync_cursor (id INTEGER PRIMARY KEY, cursor INTEGER)
        "#,
    )
    .execute(pool)
    .await?;

    // Create blob cache table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blob_cache (
            cid TEXT PRIMARY KEY,
            data BLOB NOT NULL,
            mime_type TEXT NOT NULL,
            size INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            last_accessed TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            access_count INTEGER DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Handle migration: add image columns to existing recipes table if they don't exist
    let has_image_columns = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('recipes') WHERE name IN ('image_cid', 'image_mime_type')"
    )
    .fetch_one(pool)
    .await? == 2; // Both columns exist
    
    if !has_image_columns {
        // Check which columns exist and add the missing ones
        let existing_columns: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM pragma_table_info('recipes') WHERE name IN ('image_cid', 'image_mime_type')"
        )
        .fetch_all(pool)
        .await?;
        
        if !existing_columns.contains(&"image_cid".to_string()) {
            sqlx::query("ALTER TABLE recipes ADD COLUMN image_cid TEXT")
                .execute(pool)
                .await?;
        }
        
        if !existing_columns.contains(&"image_mime_type".to_string()) {
            sqlx::query("ALTER TABLE recipes ADD COLUMN image_mime_type TEXT")
                .execute(pool)
                .await?;
        }
    }

    // Create index for recipe images (only after columns exist)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipes_image_cid ON recipes(image_cid)")
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_cursor(pool: &SqlitePool) -> anyhow::Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT cursor FROM sync_cursor WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(c,)| c))
}

pub async fn save_cursor(pool: &SqlitePool, cursor: i64) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO sync_cursor (id, cursor) VALUES (1, ?)")
        .bind(cursor)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_recipe(pool: &SqlitePool, rkey: &str, author_did: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM recipes WHERE rkey = ? AND author_did = ?")
        .bind(rkey)
        .bind(author_did)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn save_recipe(
    pool: &SqlitePool,
    uri: &str,
    author_did: &str,
    author_handle: &str,
    rkey: &str,
    name: &str,
    content: &str,
    portions: u32,
    time: u32,
    created_at: &str,
    description: Option<&str>,
    prep_time: Option<u32>,
    cook_time: Option<u32>,
    image_cid: Option<&str>,
    image_mime_type: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO recipes (author_did, rkey, uri, author_handle, name, content, portions, time, created_at, description, prep_time, cook_time, image_cid, image_mime_type)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(author_did, rkey) DO UPDATE SET
            uri = excluded.uri,
            author_handle = excluded.author_handle,
            name = excluded.name,
            content = excluded.content,
            portions = excluded.portions,
            time = excluded.time,
            created_at = excluded.created_at,
            description = excluded.description,
            prep_time = excluded.prep_time,
            cook_time = excluded.cook_time,
            image_cid = excluded.image_cid,
            image_mime_type = excluded.image_mime_type
        "#,
    )
    .bind(author_did)
    .bind(rkey)
    .bind(uri)
    .bind(author_handle)
    .bind(name)
    .bind(content)
    .bind(portions)
    .bind(time)
    .bind(created_at)
    .bind(description)
    .bind(prep_time)
    .bind(cook_time)
    .bind(image_cid)
    .bind(image_mime_type)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct SqliteRecipeDetailRow {
    rkey: String,
    author_handle: String,
    name: String,
    content: String,
    portions: i64,
    time: i64,
    created_at: String,
    description: Option<String>,
    prep_time: Option<i64>,
    cook_time: Option<i64>,
    image_cid: Option<String>,
    image_mime_type: Option<String>,
}

pub struct RecipeDetailRow {
    pub rkey: String,
    pub author_handle: String,
    pub name: String,
    pub content: String,
    pub portions: u32,
    pub time: u32,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub prep_time: Option<u32>,
    pub cook_time: Option<u32>,
    pub image_cid: Option<String>,
    pub image_mime_type: Option<String>,
}

pub async fn get_recipe(pool: &SqlitePool, author_handle: &str, rkey: &str) -> anyhow::Result<Option<RecipeDetailRow>> {
    let row = sqlx::query_as::<_, SqliteRecipeDetailRow>(
        r#"
        SELECT rkey, author_handle, name, content, portions, time, created_at, description, prep_time, cook_time, image_cid, image_mime_type
        FROM recipes
        WHERE author_handle = ? AND rkey = ? AND content IS NOT NULL
        "#,
    )
    .bind(author_handle)
    .bind(rkey)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| RecipeDetailRow {
        rkey: r.rkey,
        author_handle: r.author_handle,
        name: r.name,
        content: r.content,
        portions: r.portions as u32,
        time: r.time as u32,
        created_at: DateTime::parse_from_rfc3339(&r.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        description: r.description,
        prep_time: r.prep_time.map(|v| v as u32),
        cook_time: r.cook_time.map(|v| v as u32),
        image_cid: r.image_cid,
        image_mime_type: r.image_mime_type,
    }))
}

pub async fn get_all_recipes(pool: &SqlitePool) -> anyhow::Result<Vec<RecipeRow>> {
    let rows = sqlx::query_as::<_, SqliteRecipeRow>(
        r#"
        SELECT rkey, author_handle, name, created_at
        FROM recipes
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(RecipeRow::from).collect())
}

#[derive(sqlx::FromRow)]
struct SqliteRecipeRow {
    rkey: String,
    author_handle: String,
    name: String,
    created_at: String,
}

pub struct RecipeRow {
    pub rkey: String,
    pub author_handle: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<SqliteRecipeRow> for RecipeRow {
    fn from(row: SqliteRecipeRow) -> Self {
        RecipeRow {
            rkey: row.rkey,
            author_handle: row.author_handle,
            name: row.name,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

pub async fn upsert_user(pool: &SqlitePool, did: &str, handle: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO users (did, handle, first_login_at, last_login_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(did) DO UPDATE SET last_login_at = excluded.last_login_at
        "#,
    )
    .bind(did)
    .bind(handle)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_all_users(pool: &SqlitePool) -> anyhow::Result<Vec<UserRow>> {
    let rows = sqlx::query_as::<_, SqliteUserRow>(
        r#"
        SELECT did, handle, first_login_at
        FROM users
        ORDER BY first_login_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(UserRow::from).collect())
}

#[derive(sqlx::FromRow)]
struct SqliteUserRow {
    #[allow(dead_code)]
    did: String,
    handle: String,
    first_login_at: String,
}

pub struct UserRow {
    pub handle: String,
    pub joined_at: DateTime<Utc>,
}

impl From<SqliteUserRow> for UserRow {
    fn from(row: SqliteUserRow) -> Self {
        let joined = DateTime::parse_from_rfc3339(&row.first_login_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        UserRow {
            handle: row.handle,
            joined_at: joined,
        }
    }
}

/// Check if a user is an AtChef member (has logged in at least once)
pub async fn is_atchef_member(pool: &SqlitePool, did: &str) -> anyhow::Result<bool> {
    let result = sqlx::query_scalar::<_, String>("SELECT did FROM users WHERE did = ?")
        .bind(did)
        .fetch_optional(pool)
        .await?;
    Ok(result.is_some())
}


