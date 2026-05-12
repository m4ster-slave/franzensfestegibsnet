use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, NaiveDateTime, Utc};
use rocket::form::FromForm;
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::request::{FromRequest, Outcome, Request};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

const SESSION_COOKIE: &str = "session";
const SESSION_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub disabled: bool,
    pub avatar_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UserSummary {
    pub id: i32,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, FromForm)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, FromForm)]
pub struct LoginForm {
    pub username_or_email: String,
    pub password: String,
}

#[derive(Debug, FromForm)]
pub struct RoleForm {
    pub role: String,
}

#[derive(Debug, FromForm)]
pub struct PasswordResetForm {
    pub password: String,
}

#[derive(Debug)]
pub struct CurrentUser(pub User);

#[derive(Debug)]
pub struct AdminUser(pub User);

#[derive(Debug)]
pub struct ModeratorUser(pub User);

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn can_moderate(&self) -> bool {
        self.role == "admin" || self.role == "moderator"
    }

    pub async fn create(
        pool: &PgPool,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<User, String> {
        validate_role(role)?;
        let password_hash = hash_password(password)?;

        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(username.trim())
        .bind(email.trim())
        .bind(password_hash)
        .bind(role)
        .fetch_one(pool)
        .await
        .map_err(|err| err.to_string())
    }

    pub async fn find_for_login(
        pool: &PgPool,
        username_or_email: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT *
            FROM users
            WHERE lower(username) = lower($1) OR lower(email) = lower($1)
            "#,
        )
        .bind(username_or_email.trim())
        .fetch_optional(pool)
        .await
    }

    pub async fn all(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT *
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn summaries_by_ids(
        pool: &PgPool,
        ids: &[i32],
    ) -> Result<HashMap<i32, UserSummary>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let users = sqlx::query_as::<_, UserSummary>(
            r#"
            SELECT id, username, avatar_url
            FROM users
            WHERE id = ANY($1)
            "#,
        )
        .bind(ids.to_vec())
        .fetch_all(pool)
        .await?;

        Ok(users.into_iter().map(|user| (user.id, user)).collect())
    }

    pub async fn set_role(pool: &PgPool, id: i32, role: &str) -> Result<(), String> {
        validate_role(role)?;
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn set_disabled(pool: &PgPool, id: i32, disabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET disabled = $1 WHERE id = $2")
            .bind(disabled)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn set_password(pool: &PgPool, id: i32, password: &str) -> Result<(), String> {
        let password_hash = hash_password(password)?;
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(password_hash)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn set_avatar_url(
        pool: &PgPool,
        id: i32,
        avatar_url: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
            .bind(avatar_url)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

pub async fn bootstrap_admin(pool: &PgPool) -> Result<(), String> {
    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(pool)
        .await
        .map_err(|err| err.to_string())?;

    if admin_count > 0 {
        return Ok(());
    }

    let username = env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let email = env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.local".to_string());
    let password = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "change-me-now".to_string());

    User::create(pool, &username, &email, &password, "admin").await?;
    eprintln!(
        "Bootstrapped admin account '{}'. Change ADMIN_PASSWORD before production.",
        username
    );
    Ok(())
}

pub fn validate_role(role: &str) -> Result<(), String> {
    match role {
        "user" | "moderator" | "admin" => Ok(()),
        _ => Err("Invalid role".to_string()),
    }
}

pub fn hash_password(password: &str) -> Result<String, String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| err.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub async fn create_session(
    pool: &PgPool,
    cookies: &CookieJar<'_>,
    user_id: i32,
) -> Result<(), sqlx::Error> {
    let token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now().naive_utc() + Duration::days(SESSION_DAYS);

    sqlx::query(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(rocket::time::Duration::days(SESSION_DAYS))
        .build();
    cookies.add_private(cookie);
    Ok(())
}

pub async fn destroy_session(pool: &PgPool, cookies: &CookieJar<'_>) -> Result<(), sqlx::Error> {
    if let Some(cookie) = cookies.get_private(SESSION_COOKIE) {
        let token_hash = hash_token(cookie.value());
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(pool)
            .await?;
    }

    cookies.remove_private(Cookie::build(SESSION_COOKIE).path("/").build());
    Ok(())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

async fn load_user_from_request(request: &Request<'_>) -> Result<User, Status> {
    let pool = request
        .rocket()
        .state::<PgPool>()
        .ok_or(Status::InternalServerError)?;
    let cookies = request.cookies();
    let token = cookies
        .get_private(SESSION_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .ok_or(Status::Unauthorized)?;

    let token_hash = hash_token(&token);
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT users.*
        FROM users
        INNER JOIN sessions ON sessions.user_id = users.id
        WHERE sessions.token_hash = $1
          AND sessions.expires_at > CURRENT_TIMESTAMP
          AND users.disabled = FALSE
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| Status::InternalServerError)?
    .ok_or(Status::Unauthorized)?;

    Ok(user)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match load_user_from_request(request).await {
            Ok(user) => Outcome::Success(CurrentUser(user)),
            Err(status) => Outcome::Error((status, ())),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match load_user_from_request(request).await {
            Ok(user) if user.is_admin() => Outcome::Success(AdminUser(user)),
            Ok(_) => Outcome::Error((Status::Forbidden, ())),
            Err(status) => Outcome::Error((status, ())),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ModeratorUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match load_user_from_request(request).await {
            Ok(user) if user.can_moderate() => Outcome::Success(ModeratorUser(user)),
            Ok(_) => Outcome::Error((Status::Forbidden, ())),
            Err(status) => Outcome::Error((status, ())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_role;

    #[test]
    fn accepts_known_roles() {
        assert!(validate_role("user").is_ok());
        assert!(validate_role("moderator").is_ok());
        assert!(validate_role("admin").is_ok());
    }

    #[test]
    fn rejects_unknown_roles() {
        assert!(validate_role("owner").is_err());
    }
}
