use chrono::NaiveDateTime;
use rocket::form::FromForm;
use rocket::request::{FromRequest, Outcome, Request};
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

pub struct ClientInfo {
    pub ip: String,
    pub user_agent: String,
    pub fingerprint: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientInfo {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = request
            .client_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let user_agent = request
            .headers()
            .get_one("User-Agent")
            .unwrap_or("unknown")
            .to_string();

        let fingerprint = request
            .headers()
            .get_one("X-Fingerprint")
            .unwrap_or("")
            .to_string();

        Outcome::Success(ClientInfo {
            ip,
            user_agent,
            fingerprint,
        })
    }
}

impl Post {
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

    pub async fn get_paginated(
        pool: &PgPool,
        page: i64,
        items_per_page: i64,
    ) -> Result<(Vec<Post>, Pagination), sqlx::Error> {
        let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM posts")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);

        let total_pages = (total_count + items_per_page - 1) / items_per_page;
        let offset = (page - 1) * items_per_page;

        let posts = sqlx::query_as!(
            Post,
            r#"
            SELECT *
            FROM posts
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            items_per_page,
            offset
        )
        .fetch_all(pool)
        .await?;

        let pagination = Pagination {
            current_page: page,
            total_pages,
            items_per_page,
            total_items: total_count,
        };

        Ok((posts, pagination))
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

    pub async fn validate_client(pool: &PgPool, client: &ClientInfo) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT 
                fp.fingerprint,
                fp.post_count,
                fp.last_post_timestamp
            FROM fingerprint_details fp
            WHERE 
                fp.fingerprint = $1 
                OR (fp.ip_address = $2 AND fp.user_agent = $3)
            "#,
            client.fingerprint,
            client.ip,
            client.user_agent
        )
        .fetch_optional(pool)
        .await?;

        if let Some(record) = result {
            let ten_minutes_ago = chrono::Utc::now().naive_utc() - chrono::Duration::minutes(10);

            if record.last_post_timestamp > ten_minutes_ago {
                return Ok(false);
            }

            if record.post_count > 50 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub async fn update_client_info(pool: &PgPool, client: &ClientInfo) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO fingerprint_details 
                (fingerprint, ip_address, user_agent, last_post_timestamp, post_count)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP, 1)
            ON CONFLICT (fingerprint) DO UPDATE 
            SET 
                last_post_timestamp = CURRENT_TIMESTAMP,
                post_count = fingerprint_details.post_count + 1
            "#,
            client.fingerprint,
            client.ip,
            client.user_agent
        )
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

    pub async fn get_paginated_by_post_id(
        pool: &PgPool,
        post_id: i32,
        page: i64,
        items_per_page: i64,
    ) -> Result<(Vec<Comment>, Pagination), sqlx::Error> {
        let total_count: i64 =
            sqlx::query_scalar!("SELECT COUNT(*) FROM comments WHERE post_id = $1", post_id)
                .fetch_one(pool)
                .await?
                .unwrap_or(0);

        let total_pages = (total_count + items_per_page - 1) / items_per_page;
        let offset = (page - 1) * items_per_page;

        let comments = sqlx::query_as!(
            Comment,
            r#"
            SELECT *
            FROM comments
            WHERE post_id = $1
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#,
            post_id,
            items_per_page,
            offset
        )
        .fetch_all(pool)
        .await?;

        let pagination = Pagination {
            current_page: page,
            total_pages,
            items_per_page,
            total_items: total_count,
        };

        Ok((comments, pagination))
    }
}
