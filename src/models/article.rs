use chrono::NaiveDateTime;
use rocket::form::FromForm;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Article {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub status: String,
    pub author_id: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, FromForm)]
pub struct ArticleForm {
    pub title: String,
    pub slug: Option<String>,
    pub content: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ArticleView {
    pub article: Article,
    pub content_html: String,
}

impl Article {
    pub async fn published(pool: &PgPool) -> Result<Vec<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
            SELECT *
            FROM articles
            WHERE status = 'published'
            ORDER BY title ASC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn all(pool: &PgPool) -> Result<Vec<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
            SELECT *
            FROM articles
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_published_by_slug(
        pool: &PgPool,
        slug: &str,
    ) -> Result<Option<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
            SELECT *
            FROM articles
            WHERE slug = $1 AND status = 'published'
            "#,
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn get_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
            SELECT *
            FROM articles
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &PgPool,
        form: &ArticleForm,
        author_id: i32,
    ) -> Result<Article, String> {
        let status = normalize_article_status(&form.status)?;
        let slug = form
            .slug
            .as_deref()
            .map(slugify)
            .filter(|slug| !slug.is_empty())
            .unwrap_or_else(|| slugify(&form.title));

        if slug.is_empty() {
            return Err("Article slug cannot be empty".to_string());
        }

        sqlx::query_as::<_, Article>(
            r#"
            INSERT INTO articles (slug, title, content, status, author_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(slug)
        .bind(form.title.trim())
        .bind(&form.content)
        .bind(status)
        .bind(author_id)
        .fetch_one(pool)
        .await
        .map_err(|err| err.to_string())
    }

    pub async fn update(
        pool: &PgPool,
        current_slug: &str,
        form: &ArticleForm,
    ) -> Result<Article, String> {
        let status = normalize_article_status(&form.status)?;
        let slug = form
            .slug
            .as_deref()
            .map(slugify)
            .filter(|slug| !slug.is_empty())
            .unwrap_or_else(|| slugify(&form.title));

        if slug.is_empty() {
            return Err("Article slug cannot be empty".to_string());
        }

        sqlx::query_as::<_, Article>(
            r#"
            UPDATE articles
            SET slug = $1, title = $2, content = $3, status = $4
            WHERE slug = $5
            RETURNING *
            "#,
        )
        .bind(slug)
        .bind(form.title.trim())
        .bind(&form.content)
        .bind(status)
        .bind(current_slug)
        .fetch_one(pool)
        .await
        .map_err(|err| err.to_string())
    }

    pub async fn archive(pool: &PgPool, slug: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE articles SET status = 'archived' WHERE slug = $1")
            .bind(slug)
            .execute(pool)
            .await?;
        Ok(())
    }
}

pub fn normalize_article_status(status: &str) -> Result<&'static str, String> {
    match status {
        "draft" => Ok("draft"),
        "published" => Ok("published"),
        "archived" => Ok("archived"),
        _ => Err("Invalid article status".to_string()),
    }
}

pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugifies_article_titles() {
        assert_eq!(
            slugify("The Franzensfeste Conspiracy!"),
            "the-franzensfeste-conspiracy"
        );
    }
}
