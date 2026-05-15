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
    pub tags: Vec<String>,
    pub created_at: NaiveDateTime,
}

impl Upload {
    pub async fn all_ordered(pool: &PgPool) -> Result<Vec<Upload>, sqlx::Error> {
        sqlx::query_as::<_, Upload>(
            r#"
            SELECT *
            FROM uploads
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Upload>, sqlx::Error> {
        sqlx::query_as::<_, Upload>(
            r#"
            SELECT *
            FROM uploads
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

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

    pub async fn update_original_filename(
        pool: &PgPool,
        id: i32,
        original_filename: &str,
    ) -> Result<Upload, sqlx::Error> {
        sqlx::query_as::<_, Upload>(
            r#"
            UPDATE uploads
            SET original_filename = $1
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(original_filename)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn update_tags(
        pool: &PgPool,
        id: i32,
        tags: &[String],
    ) -> Result<Upload, sqlx::Error> {
        sqlx::query_as::<_, Upload>(
            r#"
            UPDATE uploads
            SET tags = $1
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(tags)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: i32) -> Result<Option<Upload>, sqlx::Error> {
        sqlx::query_as::<_, Upload>(
            r#"
            DELETE FROM uploads
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}
