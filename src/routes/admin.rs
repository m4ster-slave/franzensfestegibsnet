use crate::markdown::render_markdown;
use crate::models::article::{Article, ArticleForm};
use crate::models::auth::{AdminUser, ModeratorUser, PasswordResetForm, RoleForm, User};
use crate::models::forum::{
    Comment, EditComment, EditPost, ModerateContent, Post, RenderedComment, RenderedPost,
};
use crate::models::upload::Upload;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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

#[derive(Debug, Serialize)]
struct ArchiveFile {
    upload: Upload,
    markdown: String,
    size_label: String,
    tags_value: String,
    tags_search: String,
    is_image: bool,
    is_pdf: bool,
    is_text: bool,
}

#[derive(FromForm)]
pub struct ArchiveUpload<'r> {
    pub files: Vec<TempFile<'r>>,
}

#[derive(FromForm)]
pub struct RenameUploadForm {
    pub original_filename: String,
}

#[derive(FromForm)]
pub struct UpdateUploadTagsForm {
    pub tags: String,
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

#[get("/admin/archive")]
pub async fn archive_panel(db: &State<PgPool>, admin: AdminUser) -> Template {
    render_archive_panel(db.inner(), admin.0, None).await
}

#[post("/admin/archive/upload", data = "<form>")]
pub async fn upload_archive_files(
    db: &State<PgPool>,
    mut form: Form<ArchiveUpload<'_>>,
    admin: AdminUser,
) -> Result<Redirect, Template> {
    let admin_user = admin.0;
    let mut uploaded = 0;

    for file in form.files.iter_mut() {
        if file.len() == 0 {
            continue;
        }

        match persist_archive_upload(db.inner(), admin_user.id, file).await {
            Ok(()) => uploaded += 1,
            Err(status) => {
                return Err(render_archive_panel(
                    db.inner(),
                    admin_user,
                    Some(archive_upload_error(status)),
                )
                .await);
            }
        }
    }

    if uploaded == 0 {
        return Err(render_archive_panel(
            db.inner(),
            admin_user,
            Some("Choose at least one file to upload.".to_string()),
        )
        .await);
    }

    Ok(Redirect::to(uri!(archive_panel)))
}

#[post("/admin/archive/<id>/rename", data = "<form>")]
pub async fn rename_archive_file(
    db: &State<PgPool>,
    id: i32,
    form: Form<RenameUploadForm>,
    _admin: AdminUser,
) -> Redirect {
    let filename = sanitize_display_filename(&form.original_filename);
    let _ = Upload::update_original_filename(db.inner(), id, &filename).await;
    Redirect::to(uri!(archive_panel))
}

#[post("/admin/archive/<id>/tags", data = "<form>")]
pub async fn update_archive_file_tags(
    db: &State<PgPool>,
    id: i32,
    form: Form<UpdateUploadTagsForm>,
    _admin: AdminUser,
) -> Redirect {
    let tags = parse_tags(&form.tags);
    let _ = Upload::update_tags(db.inner(), id, &tags).await;
    Redirect::to(uri!(archive_panel))
}

#[post("/admin/archive/<id>/delete")]
pub async fn delete_archive_file(db: &State<PgPool>, id: i32, _admin: AdminUser) -> Redirect {
    if let Ok(Some(upload)) = Upload::find_by_id(db.inner(), id).await {
        if let Some(path) = upload_disk_path(&upload.path) {
            if let Err(error) = tokio::fs::remove_file(&path).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!(
                        "Failed to remove archive file '{}': {error}",
                        path.display()
                    );
                }
            }
        }
    }

    let _ = Upload::delete(db.inner(), id).await;
    Redirect::to(uri!(archive_panel))
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

async fn render_archive_panel(pool: &PgPool, admin: User, error: Option<String>) -> Template {
    match Upload::all_ordered(pool).await {
        Ok(files) => Template::render(
            "admin_archive",
            context! {
                files: files.into_iter().map(ArchiveFile::from).collect::<Vec<_>>(),
                current_user: admin,
                error: error,
                max_upload_mb: max_upload_bytes() / 1024 / 1024,
            },
        ),
        Err(_) => Template::render(
            "admin_archive",
            context! {
                error: error.unwrap_or_else(|| "Failed to load archive files".to_string()),
                current_user: admin,
                max_upload_mb: max_upload_bytes() / 1024 / 1024,
            },
        ),
    }
}

async fn persist_archive_upload(
    pool: &PgPool,
    owner_id: i32,
    file: &mut TempFile<'_>,
) -> Result<(), Status> {
    let allowed = allowed_archive_file(file).ok_or(Status::UnsupportedMediaType)?;
    let size_bytes = file.len();

    if size_bytes > max_upload_bytes() {
        return Err(Status::PayloadTooLarge);
    }

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let filename = format!("{}.{}", Uuid::new_v4(), allowed.extension);
    let mut destination = PathBuf::from(upload_dir);
    destination.push(&filename);

    file.move_copy_to(&destination).await.map_err(|error| {
        eprintln!(
            "Failed to save archive upload to '{}': {error}",
            destination.display()
        );
        Status::InternalServerError
    })?;

    let public_path = format!("/uploads/{}", filename);
    Upload::create(
        pool,
        owner_id,
        &public_path,
        &allowed.original_filename,
        &allowed.mime_type,
        size_bytes as i64,
    )
    .await
    .map_err(|error| {
        eprintln!("Failed to record archive upload '{}': {error}", public_path);
        Status::InternalServerError
    })?;

    Ok(())
}

impl From<Upload> for ArchiveFile {
    fn from(upload: Upload) -> Self {
        let label = markdown_label(&upload.original_filename);
        let is_image = upload.mime_type.starts_with("image/");
        let is_pdf = upload.mime_type == "application/pdf";
        let is_text = matches!(
            upload.mime_type.as_str(),
            "text/plain" | "text/markdown" | "text/csv" | "application/json"
        );
        let markdown = if is_image {
            format!("![{}]({})", label, upload.path)
        } else {
            format!("[{}]({})", label, upload.path)
        };
        let tags_value = upload.tags.join(", ");
        let tags_search = upload.tags.join(" ").to_ascii_lowercase();
        let size_label = format_size(upload.size_bytes);

        Self {
            upload,
            markdown,
            size_label,
            tags_value,
            tags_search,
            is_image,
            is_pdf,
            is_text,
        }
    }
}

struct AllowedArchiveFile {
    extension: &'static str,
    mime_type: String,
    original_filename: String,
}

fn allowed_archive_file(file: &TempFile<'_>) -> Option<AllowedArchiveFile> {
    let original_filename = file
        .raw_name()
        .map(|name| sanitize_display_filename(name.dangerous_unsafe_unsanitized_raw().as_str()))
        .unwrap_or_else(|| "archive-file".to_string());

    let extension = Path::new(&original_filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    let mime_type = file
        .content_type()
        .map(|content_type| content_type.to_string());

    if extension
        .as_deref()
        .map(is_blocked_archive_extension)
        .unwrap_or(false)
        || mime_type
            .as_deref()
            .map(is_blocked_archive_mime)
            .unwrap_or(false)
    {
        return None;
    }

    if let Some(mime_type) = mime_type.as_deref() {
        if let Some(extension) = archive_extension_for_mime(mime_type) {
            return Some(AllowedArchiveFile {
                extension,
                mime_type: mime_type.to_string(),
                original_filename,
            });
        }
    }

    let extension = extension?;
    archive_mime_for_extension(&extension).map(|mime_type| AllowedArchiveFile {
        extension: extension_to_static(&extension),
        mime_type: mime_type.to_string(),
        original_filename,
    })
}

fn sanitize_display_filename(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let filename = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .next_back()
        .unwrap_or("archive-file")
        .trim();
    let filename = filename
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    let filename = filename.trim_matches('.');

    if filename.is_empty() {
        "archive-file".to_string()
    } else {
        filename.chars().take(255).collect()
    }
}

fn upload_disk_path(public_path: &str) -> Option<PathBuf> {
    let filename = public_path.strip_prefix("/uploads/")?;

    if filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
        || filename.is_empty()
    {
        return None;
    }

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    Some(PathBuf::from(upload_dir).join(filename))
}

fn archive_upload_error(status: Status) -> String {
    if status.code == Status::PayloadTooLarge.code {
        format!(
            "A file is too large. Use files under {} MB.",
            max_upload_bytes() / 1024 / 1024
        )
    } else if status.code == Status::UnsupportedMediaType.code {
        "Unsupported file type. Use images, PDFs, text/Markdown, CSV, JSON, ZIP, or common office documents.".to_string()
    } else {
        "Archive upload failed.".to_string()
    }
}

fn format_size(size_bytes: i64) -> String {
    let size = size_bytes.max(0) as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;

    if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{} B", size_bytes.max(0))
    }
}

fn markdown_label(value: &str) -> String {
    value.replace('[', "\\[").replace(']', "\\]")
}

fn parse_tags(value: &str) -> Vec<String> {
    let mut tags = Vec::new();

    for raw_tag in value.split(',') {
        let tag = raw_tag
            .trim()
            .trim_start_matches('#')
            .chars()
            .filter(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ' ')
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");
        let tag = tag.to_ascii_lowercase();

        if !tag.is_empty() && tag.len() <= 40 && !tags.contains(&tag) {
            tags.push(tag);
        }
    }

    tags
}

fn max_upload_bytes() -> u64 {
    env::var("UPLOAD_MAX_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(12 * 1024 * 1024)
}

fn archive_extension_for_mime(mime_type: &str) -> Option<&'static str> {
    match mime_type {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "application/pdf" => Some("pdf"),
        "text/plain" => Some("txt"),
        "text/markdown" | "text/x-markdown" | "application/markdown" => Some("md"),
        "text/csv" => Some("csv"),
        "application/json" => Some("json"),
        "application/zip" | "application/x-zip-compressed" => Some("zip"),
        "application/msword" => Some("doc"),
        "application/vnd.ms-excel" => Some("xls"),
        "application/vnd.ms-powerpoint" => Some("ppt"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some("docx"),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some("xlsx"),
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Some("pptx"),
        "application/vnd.oasis.opendocument.text" => Some("odt"),
        "application/vnd.oasis.opendocument.spreadsheet" => Some("ods"),
        "application/vnd.oasis.opendocument.presentation" => Some("odp"),
        _ => None,
    }
}

fn archive_mime_for_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "pdf" => Some("application/pdf"),
        "txt" => Some("text/plain"),
        "md" | "markdown" => Some("text/markdown"),
        "csv" => Some("text/csv"),
        "json" => Some("application/json"),
        "zip" => Some("application/zip"),
        "doc" => Some("application/msword"),
        "xls" => Some("application/vnd.ms-excel"),
        "ppt" => Some("application/vnd.ms-powerpoint"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "odt" => Some("application/vnd.oasis.opendocument.text"),
        "ods" => Some("application/vnd.oasis.opendocument.spreadsheet"),
        "odp" => Some("application/vnd.oasis.opendocument.presentation"),
        _ => None,
    }
}

fn extension_to_static(extension: &str) -> &'static str {
    match extension {
        "jpeg" => "jpg",
        "markdown" => "md",
        "png" => "png",
        "jpg" => "jpg",
        "webp" => "webp",
        "gif" => "gif",
        "pdf" => "pdf",
        "txt" => "txt",
        "md" => "md",
        "csv" => "csv",
        "json" => "json",
        "zip" => "zip",
        "doc" => "doc",
        "xls" => "xls",
        "ppt" => "ppt",
        "docx" => "docx",
        "xlsx" => "xlsx",
        "pptx" => "pptx",
        "odt" => "odt",
        "ods" => "ods",
        "odp" => "odp",
        _ => "bin",
    }
}

fn is_blocked_archive_extension(extension: &str) -> bool {
    matches!(
        extension,
        "app"
            | "bat"
            | "cmd"
            | "com"
            | "dll"
            | "exe"
            | "html"
            | "htm"
            | "jar"
            | "js"
            | "mjs"
            | "php"
            | "ps1"
            | "scr"
            | "sh"
            | "svg"
            | "vbs"
            | "wasm"
    )
}

fn is_blocked_archive_mime(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/svg+xml"
            | "text/html"
            | "application/javascript"
            | "text/javascript"
            | "application/x-sh"
            | "application/x-msdownload"
            | "application/x-msdos-program"
            | "application/x-php"
    )
}
