use crate::markdown::render_markdown;
use crate::models::article::{Article, ArticleForm};
use crate::models::auth::{AdminUser, ModeratorUser, PasswordResetForm, RoleForm, User};
use crate::models::forum::{
    Comment, EditComment, EditPost, ModerateContent, Post, RenderedComment, RenderedPost,
};
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct AdminStats {
    users: i64,
    posts_visible: i64,
    posts_hidden: i64,
    posts_removed: i64,
    comments_visible: i64,
    comments_hidden: i64,
    comments_removed: i64,
    articles_published: i64,
    articles_draft: i64,
}

#[derive(Debug, Serialize)]
struct ModerationPost {
    post: Post,
    content_html: String,
    comments: Vec<ModerationComment>,
}

#[derive(Debug, Serialize)]
struct ModerationComment {
    comment: Comment,
    content_html: String,
}

#[get("/admin")]
pub async fn admin_panel(db: &State<PgPool>, admin: AdminUser) -> Template {
    let stats = load_stats(db.inner()).await.unwrap_or(AdminStats {
        users: 0,
        posts_visible: 0,
        posts_hidden: 0,
        posts_removed: 0,
        comments_visible: 0,
        comments_hidden: 0,
        comments_removed: 0,
        articles_published: 0,
        articles_draft: 0,
    });

    Template::render(
        "admin_panel",
        context! {
            stats: stats,
            current_user: admin.0,
        },
    )
}

#[get("/admin/users")]
pub async fn users_panel(db: &State<PgPool>, admin: AdminUser) -> Template {
    match User::all(db.inner()).await {
        Ok(users) => Template::render(
            "admin_users",
            context! {
                users: users,
                current_user: admin.0,
            },
        ),
        Err(_) => Template::render(
            "admin_users",
            context! {
                error: "Failed to load users",
                current_user: admin.0,
            },
        ),
    }
}

#[post("/admin/users/<id>/role", data = "<role>")]
pub async fn set_user_role(
    db: &State<PgPool>,
    id: i32,
    role: Form<RoleForm>,
    _admin: AdminUser,
) -> Redirect {
    let _ = User::set_role(db.inner(), id, &role.role).await;
    Redirect::to(uri!(users_panel))
}

#[post("/admin/users/<id>/disable")]
pub async fn disable_user(db: &State<PgPool>, id: i32, _admin: AdminUser) -> Redirect {
    let _ = User::disable(db.inner(), id).await;
    Redirect::to(uri!(users_panel))
}

#[post("/admin/users/<id>/enable")]
pub async fn enable_user(db: &State<PgPool>, id: i32, _admin: AdminUser) -> Redirect {
    let _ = User::enable(db.inner(), id).await;
    Redirect::to(uri!(users_panel))
}

#[post("/admin/users/<id>/password", data = "<form>")]
pub async fn reset_user_password(
    db: &State<PgPool>,
    id: i32,
    form: Form<PasswordResetForm>,
    _admin: AdminUser,
) -> Redirect {
    let form = form.into_inner();
    let _ = User::set_password(db.inner(), id, &form.password).await;
    Redirect::to(uri!(users_panel))
}

#[get("/admin/articles")]
pub async fn articles_panel(db: &State<PgPool>, admin: AdminUser) -> Template {
    match Article::all(db.inner()).await {
        Ok(articles) => Template::render(
            "admin_articles",
            context! {
                articles: articles,
                current_user: admin.0,
            },
        ),
        Err(_) => Template::render(
            "admin_articles",
            context! {
                error: "Failed to load articles",
                current_user: admin.0,
            },
        ),
    }
}

#[get("/admin/articles/new")]
pub async fn new_article(admin: AdminUser) -> Template {
    Template::render(
        "admin_article_edit",
        context! {
            current_user: admin.0,
            action: "/admin/articles",
            is_new: true,
        },
    )
}

#[post("/admin/articles", data = "<form>")]
pub async fn create_article(
    db: &State<PgPool>,
    form: Form<ArticleForm>,
    admin: AdminUser,
) -> Result<Redirect, Template> {
    match Article::create(db.inner(), &form.into_inner(), admin.0.id).await {
        Ok(article) => Ok(Redirect::to(uri!(edit_article(slug = article.slug)))),
        Err(error) => Err(Template::render(
            "admin_article_edit",
            context! {
                error: error,
                current_user: admin.0,
                action: "/admin/articles",
                is_new: true,
            },
        )),
    }
}

#[get("/admin/articles/<slug>/edit")]
pub async fn edit_article(
    db: &State<PgPool>,
    slug: &str,
    admin: AdminUser,
) -> Result<Template, Redirect> {
    match Article::get_by_slug(db.inner(), slug).await {
        Ok(Some(article)) => Ok(Template::render(
            "admin_article_edit",
            context! {
                article: article,
                current_user: admin.0,
                action: format!("/admin/articles/{}", slug),
                is_new: false,
            },
        )),
        _ => Err(Redirect::to(uri!(articles_panel))),
    }
}

#[post("/admin/articles/<slug>", data = "<form>")]
pub async fn update_article(
    db: &State<PgPool>,
    slug: &str,
    form: Form<ArticleForm>,
    admin: AdminUser,
) -> Result<Redirect, Template> {
    match Article::update(db.inner(), slug, &form.into_inner()).await {
        Ok(article) => Ok(Redirect::to(uri!(edit_article(slug = article.slug)))),
        Err(error) => Err(Template::render(
            "admin_article_edit",
            context! {
                error: error,
                current_user: admin.0,
                action: format!("/admin/articles/{}", slug),
                is_new: false,
            },
        )),
    }
}

#[post("/admin/articles/<slug>/delete")]
pub async fn delete_article(db: &State<PgPool>, slug: &str, _admin: AdminUser) -> Redirect {
    let _ = Article::archive(db.inner(), slug).await;
    Redirect::to(uri!(articles_panel))
}

#[get("/admin/moderation")]
pub async fn moderation_panel(db: &State<PgPool>, moderator: ModeratorUser) -> Template {
    match load_moderation_posts(db.inner()).await {
        Ok(posts) => Template::render(
            "admin_moderation",
            context! {
                posts: posts,
                current_user: moderator.0,
            },
        ),
        Err(_) => Template::render(
            "admin_moderation",
            context! {
                error: "Failed to load moderation queue",
                current_user: moderator.0,
            },
        ),
    }
}

#[get("/admin/moderation/posts")]
pub async fn moderation_posts(_moderator: ModeratorUser) -> Redirect {
    Redirect::to(uri!(moderation_panel))
}

#[get("/admin/moderation/comments")]
pub async fn moderation_comments(_moderator: ModeratorUser) -> Redirect {
    Redirect::to(uri!(moderation_panel))
}

#[post("/admin/posts/<id>/moderate", data = "<form>")]
pub async fn moderate_post(
    db: &State<PgPool>,
    id: i32,
    form: Form<ModerateContent>,
    moderator: ModeratorUser,
) -> Redirect {
    let form = form.into_inner();
    let _ = Post::moderate(
        db.inner(),
        id,
        &form.status,
        form.moderator_note.as_deref(),
        moderator.0.id,
    )
    .await;
    Redirect::to(uri!(moderation_panel))
}

#[post("/admin/posts/<id>/lock")]
pub async fn lock_post(db: &State<PgPool>, id: i32, _moderator: ModeratorUser) -> Redirect {
    let _ = Post::set_locked(db.inner(), id, true).await;
    Redirect::to(uri!(moderation_panel))
}

#[post("/admin/posts/<id>/unlock")]
pub async fn unlock_post(db: &State<PgPool>, id: i32, _moderator: ModeratorUser) -> Redirect {
    let _ = Post::set_locked(db.inner(), id, false).await;
    Redirect::to(uri!(moderation_panel))
}

#[post("/admin/posts/<id>/delete")]
pub async fn delete_post(db: &State<PgPool>, id: i32, moderator: ModeratorUser) -> Redirect {
    let _ = Post::moderate(
        db.inner(),
        id,
        "removed",
        Some("Soft-deleted from admin"),
        moderator.0.id,
    )
    .await;
    Redirect::to(uri!(moderation_panel))
}

#[get("/admin/posts/<id>/edit")]
pub async fn edit_post_panel(
    db: &State<PgPool>,
    id: i32,
    moderator: ModeratorUser,
) -> Result<Template, Redirect> {
    let post_result = Post::get_by_id(db.inner(), id).await;
    let comments_result = Comment::get_all_by_post_id(db.inner(), id).await;

    match (post_result, comments_result) {
        (Ok(post), Ok(comments)) => {
            let rendered_post = RenderedPost {
                author: None,
                content_html: render_markdown(&post.content),
                post,
            };
            let rendered_comments: Vec<RenderedComment> = comments
                .into_iter()
                .map(|comment| RenderedComment {
                    author: None,
                    content_html: render_markdown(&comment.content),
                    comment,
                })
                .collect();
            Ok(Template::render(
                "admin_edit_post",
                context! {
                    rendered_post: rendered_post,
                    comments: rendered_comments,
                    current_user: moderator.0,
                },
            ))
        }
        _ => Err(Redirect::to(uri!(moderation_panel))),
    }
}

#[post("/admin/posts/<id>/edit", data = "<post>")]
pub async fn update_post(
    db: &State<PgPool>,
    id: i32,
    post: Form<EditPost>,
    _moderator: ModeratorUser,
) -> Redirect {
    let _ = Post::update(db.inner(), id, &post.into_inner()).await;
    Redirect::to(uri!(edit_post_panel(id)))
}

#[post("/admin/comments/<id>/moderate", data = "<form>")]
pub async fn moderate_comment(
    db: &State<PgPool>,
    id: i32,
    form: Form<ModerateContent>,
    moderator: ModeratorUser,
) -> Redirect {
    let form = form.into_inner();
    let _ = Comment::moderate(
        db.inner(),
        id,
        &form.status,
        form.moderator_note.as_deref(),
        moderator.0.id,
    )
    .await;
    Redirect::to(uri!(moderation_panel))
}

#[post("/admin/comments/<id>/delete")]
pub async fn delete_comment(db: &State<PgPool>, id: i32, moderator: ModeratorUser) -> Redirect {
    let post_id = Comment::post_id(db.inner(), id).await.ok().flatten();
    let _ = Comment::moderate(
        db.inner(),
        id,
        "removed",
        Some("Soft-deleted from admin"),
        moderator.0.id,
    )
    .await;

    match post_id {
        Some(post_id) => Redirect::to(uri!(edit_post_panel(id = post_id))),
        None => Redirect::to(uri!(moderation_panel)),
    }
}

#[post("/admin/comments/<id>/edit", data = "<comment>")]
pub async fn edit_comment(
    db: &State<PgPool>,
    id: i32,
    comment: Form<EditComment>,
    _moderator: ModeratorUser,
) -> Redirect {
    let post_id = Comment::post_id(db.inner(), id).await.ok().flatten();
    let _ = Comment::update(db.inner(), id, &comment.into_inner()).await;

    match post_id {
        Some(post_id) => Redirect::to(uri!(edit_post_panel(id = post_id))),
        None => Redirect::to(uri!(moderation_panel)),
    }
}

async fn load_moderation_posts(pool: &PgPool) -> Result<Vec<ModerationPost>, sqlx::Error> {
    let posts = Post::get_all(pool).await?;
    let comments = Comment::get_all(pool).await?;
    let mut comments_by_post: HashMap<i32, Vec<ModerationComment>> = HashMap::new();

    for comment in comments {
        comments_by_post
            .entry(comment.post_id)
            .or_default()
            .push(ModerationComment {
                content_html: render_markdown(&comment.content),
                comment,
            });
    }

    Ok(posts
        .into_iter()
        .map(|post| ModerationPost {
            content_html: render_markdown(&post.content),
            comments: comments_by_post.remove(&post.id).unwrap_or_default(),
            post,
        })
        .collect())
}

async fn load_stats(pool: &PgPool) -> Result<AdminStats, sqlx::Error> {
    Ok(AdminStats {
        users: count(pool, "SELECT COUNT(*) FROM users").await?,
        posts_visible: count(pool, "SELECT COUNT(*) FROM posts WHERE status = 'visible'").await?,
        posts_hidden: count(pool, "SELECT COUNT(*) FROM posts WHERE status = 'hidden'").await?,
        posts_removed: count(pool, "SELECT COUNT(*) FROM posts WHERE status = 'removed'").await?,
        comments_visible: count(
            pool,
            "SELECT COUNT(*) FROM comments WHERE status = 'visible'",
        )
        .await?,
        comments_hidden: count(
            pool,
            "SELECT COUNT(*) FROM comments WHERE status = 'hidden'",
        )
        .await?,
        comments_removed: count(
            pool,
            "SELECT COUNT(*) FROM comments WHERE status = 'removed'",
        )
        .await?,
        articles_published: count(
            pool,
            "SELECT COUNT(*) FROM articles WHERE status = 'published'",
        )
        .await?,
        articles_draft: count(pool, "SELECT COUNT(*) FROM articles WHERE status = 'draft'").await?,
    })
}

async fn count(pool: &PgPool, sql: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(sql).fetch_one(pool).await
}
