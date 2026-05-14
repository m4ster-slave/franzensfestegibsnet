use crate::models::auth::UserSummary;
use chrono::NaiveDateTime;
use rocket::form::FromForm;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub image_url: Option<String>,
    pub author_id: Option<i32>,
    pub status: String,
    pub moderator_note: Option<String>,
    pub moderated_by: Option<i32>,
    pub moderated_at: Option<NaiveDateTime>,
    pub locked: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub content: String,
    pub author_id: Option<i32>,
    pub status: String,
    pub moderator_note: Option<String>,
    pub moderated_by: Option<i32>,
    pub moderated_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct RenderedPost {
    pub post: Post,
    pub content_html: String,
    pub author: Option<UserSummary>,
}

#[derive(Debug, Serialize)]
pub struct RenderedComment {
    pub comment: Comment,
    pub content_html: String,
    pub author: Option<UserSummary>,
}

#[derive(Debug, Serialize)]
pub struct PostListItem {
    pub post: Post,
    pub author: Option<UserSummary>,
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub current_page: i64,
    pub total_pages: i64,
    pub items_per_page: i64,
    pub total_items: i64,
}

#[derive(FromForm)]
pub struct CreatePost {
    pub title: String,
    pub content: String,
}

#[derive(FromForm)]
pub struct CreatePostFingerprint {
    pub title: String,
    pub content: String,
    pub fingerprint: String,
}

#[derive(FromForm)]
pub struct CreateComment {
    pub content: String,
}

#[derive(FromForm)]
pub struct EditPost {
    pub title: String,
    pub content: String,
}

#[derive(FromForm)]
pub struct EditComment {
    pub content: String,
}

#[derive(FromForm)]
pub struct ModerateContent {
    pub status: String,
    pub moderator_note: Option<String>,
}

impl Post {
    pub async fn get_all(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>(
            r#"
            SELECT *
            FROM posts
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &PgPool,
        post: CreatePost,
        author_id: i32,
    ) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (title, content, author_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(post.title.trim())
        .bind(post.content.trim())
        .bind(author_id)
        .fetch_one(pool)
        .await
    }

    pub async fn get_paginated(
        pool: &PgPool,
        page: i64,
        items_per_page: i64,
    ) -> Result<(Vec<Post>, Pagination), sqlx::Error> {
        let total_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE status = 'visible'")
                .fetch_one(pool)
                .await?;

        let total_pages = (total_count + items_per_page - 1) / items_per_page;
        let offset = (page - 1) * items_per_page;

        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT *
            FROM posts
            WHERE status = 'visible'
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(items_per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok((
            posts,
            Pagination::new(page, total_pages, items_per_page, total_count),
        ))
    }

    pub async fn get_by_id(pool: &PgPool, id: i32) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
    }

    pub async fn get_visible_by_id(pool: &PgPool, id: i32) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1 AND status = 'visible'")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn update(pool: &PgPool, id: i32, post: &EditPost) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE posts SET title = $1, content = $2 WHERE id = $3")
            .bind(post.title.trim())
            .bind(post.content.trim())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn moderate(
        pool: &PgPool,
        id: i32,
        status: &str,
        moderator_note: Option<&str>,
        moderator_id: i32,
    ) -> Result<(), String> {
        validate_content_status(status)?;
        sqlx::query(
            r#"
            UPDATE posts
            SET status = $1, moderator_note = $2, moderated_by = $3, moderated_at = CURRENT_TIMESTAMP
            WHERE id = $4
            "#,
        )
        .bind(status)
        .bind(moderator_note)
        .bind(moderator_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn set_locked(pool: &PgPool, id: i32, locked: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE posts SET locked = $1 WHERE id = $2")
            .bind(locked)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl Comment {
    pub async fn create(
        pool: &PgPool,
        post_id: i32,
        comment: CreateComment,
        author_id: i32,
    ) -> Result<Comment, sqlx::Error> {
        sqlx::query_as::<_, Comment>(
            r#"
            INSERT INTO comments (post_id, content, author_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(comment.content.trim())
        .bind(author_id)
        .fetch_one(pool)
        .await
    }

    pub async fn get_paginated_by_post_id(
        pool: &PgPool,
        post_id: i32,
        page: i64,
        items_per_page: i64,
    ) -> Result<(Vec<Comment>, Pagination), sqlx::Error> {
        let total_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM comments WHERE post_id = $1 AND status = 'visible'",
        )
        .bind(post_id)
        .fetch_one(pool)
        .await?;

        let total_pages = (total_count + items_per_page - 1) / items_per_page;
        let offset = (page - 1) * items_per_page;

        let comments = sqlx::query_as::<_, Comment>(
            r#"
            SELECT *
            FROM comments
            WHERE post_id = $1 AND status = 'visible'
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(post_id)
        .bind(items_per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok((
            comments,
            Pagination::new(page, total_pages, items_per_page, total_count),
        ))
    }

    pub async fn get_all_by_post_id(
        pool: &PgPool,
        post_id: i32,
    ) -> Result<Vec<Comment>, sqlx::Error> {
        sqlx::query_as::<_, Comment>(
            r#"
            SELECT *
            FROM comments
            WHERE post_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<Comment>, sqlx::Error> {
        sqlx::query_as::<_, Comment>(
            r#"
            SELECT *
            FROM comments
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(pool: &PgPool, id: i32, comment: &EditComment) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE comments SET content = $1 WHERE id = $2")
            .bind(comment.content.trim())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn moderate(
        pool: &PgPool,
        id: i32,
        status: &str,
        moderator_note: Option<&str>,
        moderator_id: i32,
    ) -> Result<(), String> {
        validate_content_status(status)?;
        sqlx::query(
            r#"
            UPDATE comments
            SET status = $1, moderator_note = $2, moderated_by = $3, moderated_at = CURRENT_TIMESTAMP
            WHERE id = $4
            "#,
        )
        .bind(status)
        .bind(moderator_note)
        .bind(moderator_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn post_id(pool: &PgPool, id: i32) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT post_id FROM comments WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

impl Pagination {
    fn new(current_page: i64, total_pages: i64, items_per_page: i64, total_items: i64) -> Self {
        Self {
            current_page,
            total_pages,
            items_per_page,
            total_items,
        }
    }
}

pub fn validate_content_status(status: &str) -> Result<(), String> {
    match status {
        "visible" | "hidden" | "removed" => Ok(()),
        _ => Err("Invalid moderation status".to_string()),
    }
}
