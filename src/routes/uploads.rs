use crate::models::auth::{CurrentUser, User};
use crate::models::upload::Upload;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::ExtendedColorType;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;
use sqlx::PgPool;
use std::env;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const AVATAR_SIZE: u32 = 256;

#[derive(FromForm)]
pub struct ImageUpload<'r> {
    pub file: TempFile<'r>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub path: String,
}

#[post("/uploads/images", data = "<form>")]
pub async fn upload_image(
    db: &State<PgPool>,
    current_user: CurrentUser,
    mut form: Form<ImageUpload<'_>>,
) -> Result<Json<UploadResponse>, Status> {
    let public_path = persist_image_upload(db.inner(), current_user.0.id, &mut form.file).await?;
    Ok(Json(UploadResponse { path: public_path }))
}

#[post("/profile/avatar", data = "<form>")]
pub async fn update_profile_avatar(
    db: &State<PgPool>,
    current_user: CurrentUser,
    mut form: Form<ImageUpload<'_>>,
) -> Result<Redirect, Status> {
    let public_path = persist_avatar_upload(db.inner(), current_user.0.id, &mut form.file).await?;
    User::set_avatar_url(db.inner(), current_user.0.id, Some(&public_path))
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Redirect::to("/profile"))
}

#[post("/profile/avatar/remove")]
pub async fn remove_profile_avatar(
    db: &State<PgPool>,
    current_user: CurrentUser,
) -> Result<Redirect, Status> {
    User::set_avatar_url(db.inner(), current_user.0.id, None)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Redirect::to("/profile"))
}

async fn persist_avatar_upload(
    db: &PgPool,
    owner_id: i32,
    file: &mut TempFile<'_>,
) -> Result<String, Status> {
    let mime_type = file
        .content_type()
        .map(|content_type| content_type.to_string())
        .ok_or(Status::BadRequest)?;

    extension_for_mime(&mime_type).ok_or(Status::UnsupportedMediaType)?;
    let max_size = max_upload_bytes();
    let size_bytes = file.len();

    if size_bytes > max_size {
        return Err(Status::PayloadTooLarge);
    }

    let original_filename = file
        .raw_name()
        .map(|name| name.dangerous_unsafe_unsanitized_raw().to_string())
        .unwrap_or_else(|| "avatar".to_string());

    let mut upload_bytes = Vec::with_capacity(size_bytes as usize);
    file.open()
        .await
        .map_err(|_| Status::InternalServerError)?
        .read_to_end(&mut upload_bytes)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let webp_bytes = tokio::task::spawn_blocking(move || encode_avatar_webp(&upload_bytes))
        .await
        .map_err(|_| Status::InternalServerError)??;

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let filename = format!("{}.webp", Uuid::new_v4());
    let mut destination = PathBuf::from(upload_dir);
    destination.push(&filename);

    tokio::fs::write(&destination, &webp_bytes)
        .await
        .map_err(|error| {
            eprintln!(
                "Failed to save avatar upload to '{}': {error}",
                destination.display()
            );
            Status::InternalServerError
        })?;

    let public_path = format!("/uploads/{}", filename);

    Upload::create(
        db,
        owner_id,
        &public_path,
        &original_filename,
        "image/webp",
        webp_bytes.len() as i64,
    )
    .await
    .map_err(|error| {
        eprintln!("Failed to record avatar upload '{}': {error}", public_path);
        Status::InternalServerError
    })?;

    Ok(public_path)
}

async fn persist_image_upload(
    db: &PgPool,
    owner_id: i32,
    file: &mut TempFile<'_>,
) -> Result<String, Status> {
    let mime_type = file
        .content_type()
        .map(|content_type| content_type.to_string())
        .ok_or(Status::BadRequest)?;

    let extension = extension_for_mime(&mime_type).ok_or(Status::UnsupportedMediaType)?;
    let max_size = max_upload_bytes();
    let size_bytes = file.len();

    if size_bytes > max_size {
        return Err(Status::PayloadTooLarge);
    }

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    let mut destination = PathBuf::from(upload_dir);
    destination.push(&filename);

    let original_filename = file
        .raw_name()
        .map(|name| name.dangerous_unsafe_unsanitized_raw().to_string())
        .unwrap_or_else(|| filename.clone());

    file.move_copy_to(&destination).await.map_err(|error| {
        eprintln!(
            "Failed to save upload to '{}': {error}",
            destination.display()
        );
        Status::InternalServerError
    })?;

    let public_path = format!("/uploads/{}", filename);

    Upload::create(
        db,
        owner_id,
        &public_path,
        &original_filename,
        &mime_type,
        size_bytes as i64,
    )
    .await
    .map_err(|error| {
        eprintln!("Failed to record upload '{}': {error}", public_path);
        Status::InternalServerError
    })?;

    Ok(public_path)
}

fn encode_avatar_webp(bytes: &[u8]) -> Result<Vec<u8>, Status> {
    let image = image::load_from_memory(bytes).map_err(|_| Status::UnsupportedMediaType)?;
    let avatar = image
        .resize_to_fill(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3)
        .to_rgba8();
    let mut encoded = Vec::new();

    WebPEncoder::new_lossless(&mut encoded)
        .encode(
            avatar.as_raw(),
            AVATAR_SIZE,
            AVATAR_SIZE,
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| {
            eprintln!("Failed to encode avatar as WebP: {error}");
            Status::InternalServerError
        })?;

    Ok(encoded)
}

fn max_upload_bytes() -> u64 {
    env::var("UPLOAD_MAX_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(12 * 1024 * 1024)
}

fn extension_for_mime(mime_type: &str) -> Option<&'static str> {
    match mime_type {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}
