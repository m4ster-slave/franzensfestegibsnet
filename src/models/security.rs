use chrono::{Duration, NaiveDateTime, Utc};
use rocket::request::{FromRequest, Outcome, Request};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

const POST_10M_LIMIT: i64 = 4;
const POST_24H_LIMIT: i64 = 15;
const COMMENT_10M_LIMIT: i64 = 12;
const COMMENT_24H_LIMIT: i64 = 50;
const COMBINED_10M_LIMIT: i64 = 15;
const POST_COOLDOWN_SECONDS: i64 = 120;
const COMMENT_COOLDOWN_SECONDS: i64 = 20;

const LOGIN_FAILURE_LIMIT: i64 = 5;
const REGISTER_LIMIT: i64 = 3;

#[derive(Debug)]
pub struct ClientIp(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = request
            .client_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Outcome::Success(ClientIp(ip))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForumAction {
    Post,
    Comment,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allow,
    SlowDown,
    DisableAccount,
}

#[derive(Debug)]
pub struct ForumActivity {
    pub posts_10m: i64,
    pub posts_24h: i64,
    pub comments_10m: i64,
    pub comments_24h: i64,
    pub combined_10m: i64,
    pub seconds_since_last_post: Option<i64>,
    pub seconds_since_last_comment: Option<i64>,
}

pub fn evaluate_forum_activity(action: ForumAction, activity: &ForumActivity) -> RateLimitDecision {
    let would_disable = match action {
        ForumAction::Post => {
            activity.posts_10m + 1 >= POST_10M_LIMIT
                || activity.posts_24h + 1 >= POST_24H_LIMIT
                || activity.combined_10m + 1 >= COMBINED_10M_LIMIT
        }
        ForumAction::Comment => {
            activity.comments_10m + 1 >= COMMENT_10M_LIMIT
                || activity.comments_24h + 1 >= COMMENT_24H_LIMIT
                || activity.combined_10m + 1 >= COMBINED_10M_LIMIT
        }
    };

    if would_disable {
        return RateLimitDecision::DisableAccount;
    }

    let inside_cooldown = match action {
        ForumAction::Post => activity
            .seconds_since_last_post
            .map(|seconds| seconds < POST_COOLDOWN_SECONDS)
            .unwrap_or(false),
        ForumAction::Comment => activity
            .seconds_since_last_comment
            .map(|seconds| seconds < COMMENT_COOLDOWN_SECONDS)
            .unwrap_or(false),
    };

    if inside_cooldown {
        RateLimitDecision::SlowDown
    } else {
        RateLimitDecision::Allow
    }
}

pub async fn check_forum_rate_limit(
    pool: &PgPool,
    user_id: i32,
    action: ForumAction,
) -> Result<RateLimitDecision, sqlx::Error> {
    let activity = load_forum_activity(pool, user_id).await?;
    Ok(evaluate_forum_activity(action, &activity))
}

async fn load_forum_activity(pool: &PgPool, user_id: i32) -> Result<ForumActivity, sqlx::Error> {
    let now = Utc::now().naive_utc();
    let ten_minutes_ago = now - Duration::minutes(10);
    let day_ago = now - Duration::hours(24);

    let posts_10m = count_since(pool, "posts", user_id, ten_minutes_ago).await?;
    let posts_24h = count_since(pool, "posts", user_id, day_ago).await?;
    let comments_10m = count_since(pool, "comments", user_id, ten_minutes_ago).await?;
    let comments_24h = count_since(pool, "comments", user_id, day_ago).await?;
    let last_post = last_created_at(pool, "posts", user_id).await?;
    let last_comment = last_created_at(pool, "comments", user_id).await?;

    Ok(ForumActivity {
        posts_10m,
        posts_24h,
        comments_10m,
        comments_24h,
        combined_10m: posts_10m + comments_10m,
        seconds_since_last_post: seconds_since(now, last_post),
        seconds_since_last_comment: seconds_since(now, last_comment),
    })
}

async fn count_since(
    pool: &PgPool,
    table: &str,
    user_id: i32,
    since: NaiveDateTime,
) -> Result<i64, sqlx::Error> {
    let sql = format!(
        "SELECT COUNT(*) FROM {} WHERE author_id = $1 AND created_at >= $2",
        table
    );

    sqlx::query_scalar(&sql)
        .bind(user_id)
        .bind(since)
        .fetch_one(pool)
        .await
}

async fn last_created_at(
    pool: &PgPool,
    table: &str,
    user_id: i32,
) -> Result<Option<NaiveDateTime>, sqlx::Error> {
    let sql = format!("SELECT MAX(created_at) FROM {} WHERE author_id = $1", table);

    sqlx::query_scalar(&sql).bind(user_id).fetch_one(pool).await
}

fn seconds_since(now: NaiveDateTime, timestamp: Option<NaiveDateTime>) -> Option<i64> {
    timestamp.map(|timestamp| (now - timestamp).num_seconds())
}

pub async fn login_is_rate_limited(
    pool: &PgPool,
    ip: &str,
    identifier: &str,
) -> Result<bool, sqlx::Error> {
    let identifier_hash = hash_identifier(identifier);
    let since = Utc::now().naive_utc() - Duration::minutes(15);

    let failures: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM auth_attempts
        WHERE action = 'login'
          AND ip_address = $1
          AND identifier_hash = $2
          AND success = FALSE
          AND created_at >= $3
        "#,
    )
    .bind(ip)
    .bind(identifier_hash)
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(failures >= LOGIN_FAILURE_LIMIT)
}

pub async fn register_is_rate_limited(pool: &PgPool, ip: &str) -> Result<bool, sqlx::Error> {
    let since = Utc::now().naive_utc() - Duration::hours(1);

    let attempts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM auth_attempts
        WHERE action = 'register'
          AND ip_address = $1
          AND created_at >= $2
        "#,
    )
    .bind(ip)
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(attempts >= REGISTER_LIMIT)
}

pub async fn record_auth_attempt(
    pool: &PgPool,
    action: &str,
    ip: &str,
    identifier: Option<&str>,
    success: bool,
) -> Result<(), sqlx::Error> {
    let identifier_hash = identifier.map(hash_identifier);

    sqlx::query(
        r#"
        INSERT INTO auth_attempts (action, ip_address, identifier_hash, success)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(action)
    .bind(ip)
    .bind(identifier_hash)
    .bind(success)
    .execute(pool)
    .await?;

    Ok(())
}

fn hash_identifier(identifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(identifier.trim().to_ascii_lowercase().as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_forum_activity, ForumAction, ForumActivity, RateLimitDecision,
        COMMENT_COOLDOWN_SECONDS, POST_COOLDOWN_SECONDS,
    };

    fn quiet_activity() -> ForumActivity {
        ForumActivity {
            posts_10m: 0,
            posts_24h: 0,
            comments_10m: 0,
            comments_24h: 0,
            combined_10m: 0,
            seconds_since_last_post: None,
            seconds_since_last_comment: None,
        }
    }

    #[test]
    fn allows_normal_forum_activity() {
        assert_eq!(
            evaluate_forum_activity(ForumAction::Post, &quiet_activity()),
            RateLimitDecision::Allow
        );
    }

    #[test]
    fn slows_down_rapid_posts_without_disabling() {
        let mut activity = quiet_activity();
        activity.seconds_since_last_post = Some(POST_COOLDOWN_SECONDS - 1);

        assert_eq!(
            evaluate_forum_activity(ForumAction::Post, &activity),
            RateLimitDecision::SlowDown
        );
    }

    #[test]
    fn slows_down_rapid_comments_without_disabling() {
        let mut activity = quiet_activity();
        activity.seconds_since_last_comment = Some(COMMENT_COOLDOWN_SECONDS - 1);

        assert_eq!(
            evaluate_forum_activity(ForumAction::Comment, &activity),
            RateLimitDecision::SlowDown
        );
    }

    #[test]
    fn disables_when_post_threshold_would_be_reached() {
        let mut activity = quiet_activity();
        activity.posts_10m = 3;
        activity.combined_10m = 3;

        assert_eq!(
            evaluate_forum_activity(ForumAction::Post, &activity),
            RateLimitDecision::DisableAccount
        );
    }

    #[test]
    fn disables_when_comment_threshold_would_be_reached() {
        let mut activity = quiet_activity();
        activity.comments_10m = 11;
        activity.combined_10m = 11;

        assert_eq!(
            evaluate_forum_activity(ForumAction::Comment, &activity),
            RateLimitDecision::DisableAccount
        );
    }

    #[test]
    fn disables_when_combined_threshold_would_be_reached() {
        let mut activity = quiet_activity();
        activity.posts_10m = 7;
        activity.comments_10m = 7;
        activity.combined_10m = 14;

        assert_eq!(
            evaluate_forum_activity(ForumAction::Comment, &activity),
            RateLimitDecision::DisableAccount
        );
    }
}
