use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recipes (
            id TEXT PRIMARY KEY,
            uri TEXT NOT NULL,
            author_did TEXT NOT NULL,
            author_handle TEXT NOT NULL,
            rkey TEXT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL
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

    Ok(())
}

pub async fn save_recipe(
    pool: &SqlitePool,
    id: &str,
    uri: &str,
    author_did: &str,
    author_handle: &str,
    rkey: &str,
    name: &str,
    created_at: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO recipes (id, uri, author_did, author_handle, rkey, name, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(uri)
    .bind(author_did)
    .bind(author_handle)
    .bind(rkey)
    .bind(name)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(())
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
