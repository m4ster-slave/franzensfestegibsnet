use crate::models::auth::CurrentUser;
use rocket::fs::NamedFile;
use rocket_dyn_templates::{context, Template};
use std::path::Path;

#[get("/")]
pub async fn root(current_user: Option<CurrentUser>) -> Template {
    Template::render(
        "index",
        context! {
            current_user: current_user.map(|user| user.0),
        },
    )
}

#[get("/robots.txt")]
pub async fn robots() -> Option<NamedFile> {
    NamedFile::open(Path::new("public/robots.txt")).await.ok()
}

#[get("/sitemap.xml")]
pub async fn sitemap() -> Option<NamedFile> {
    NamedFile::open(Path::new("public/sitemap.xml")).await.ok()
}
