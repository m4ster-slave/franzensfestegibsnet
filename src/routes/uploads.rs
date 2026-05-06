use crate::models::auth::CurrentUser;
use crate::models::upload::Upload;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;
use sqlx::PgPool;
use std::env;
use std::path::PathBuf;
use uuid::Uuid;

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
    let mime_type = form
        .file
        .content_type()
        .map(|content_type| content_type.to_string())
        .ok_or(Status::BadRequest)?;

    let extension = extension_for_mime(&mime_type).ok_or(Status::UnsupportedMediaType)?;
    let max_size = env::var("UPLOAD_MAX_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5 * 1024 * 1024);

    if form.file.len() > max_size {
        return Err(Status::PayloadTooLarge);
    }

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    let mut destination = PathBuf::from(upload_dir);
    destination.push(&filename);

    form.file
        .persist_to(&destination)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let public_path = format!("/uploads/{}", filename);
    let original_filename = form
        .file
        .raw_name()
        .map(|name| name.dangerous_unsafe_unsanitized_raw().to_string())
        .unwrap_or_else(|| filename.clone());

    Upload::create(
        db.inner(),
        current_user.0.id,
        &public_path,
        &original_filename,
        &mime_type,
        form.file.len() as i64,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(UploadResponse { path: public_path }))
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
