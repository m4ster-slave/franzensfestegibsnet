use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize, FromRow)]
pub struct Upload {
    pub id: i32,
    pub owner_id: Option<i32>,
    pub path: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: NaiveDateTime,
}

impl Upload {
    pub async fn create(
        pool: &PgPool,
        owner_id: i32,
        path: &str,
        original_filename: &str,
        mime_type: &str,
        size_bytes: i64,
    ) -> Result<Upload, sqlx::Error> {
        sqlx::query_as::<_, Upload>(
            r#"
            INSERT INTO uploads (owner_id, path, original_filename, mime_type, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(owner_id)
        .bind(path)
        .bind(original_filename)
        .bind(mime_type)
        .bind(size_bytes)
        .fetch_one(pool)
        .await
    }
}
