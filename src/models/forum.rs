use chrono::NaiveDateTime;
use rocket::form::FromForm;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub image_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(FromForm)]
pub struct CreatePost {
    pub title: String,
    pub content: String,
    pub image_url: Option<String>,
}

#[derive(FromForm)]
pub struct CreatePostFingerprint {
    pub title: String,
    pub content: String,
    pub image_url: Option<String>,
    pub fingerprint: String,
}

#[derive(FromForm)]
pub struct CreateComment {
    pub content: String,
}

impl Post {
    pub async fn create(pool: &PgPool, post: CreatePost) -> Result<Post, sqlx::Error> {
        sqlx::query_as!(
            Post,
            r#"
            INSERT INTO posts (title, content, image_url)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            post.title,
            post.content,
            post.image_url,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as!(
            Post,
            r#"
            SELECT *
            FROM posts
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_by_id(pool: &PgPool, id: i32) -> Result<Post, sqlx::Error> {
        sqlx::query_as!(
            Post,
            r#"
            SELECT *
            FROM posts
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }
}

impl Comment {
    pub async fn create(
        pool: &PgPool,
        post_id: i32,
        comment: CreateComment,
    ) -> Result<Comment, sqlx::Error> {
        sqlx::query_as!(
            Comment,
            r#"
            INSERT INTO comments (post_id, content)
            VALUES ($1, $2)
            RETURNING *
            "#,
            post_id,
            comment.content,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_by_post_id(pool: &PgPool, post_id: i32) -> Result<Vec<Comment>, sqlx::Error> {
        sqlx::query_as!(
            Comment,
            r#"
            SELECT *
            FROM comments
            WHERE post_id = $1
            ORDER BY created_at ASC
            "#,
            post_id
        )
        .fetch_all(pool)
        .await
    }
}
