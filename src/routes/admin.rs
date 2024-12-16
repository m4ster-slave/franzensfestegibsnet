use crate::models::forum::Post;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::env;

#[derive(FromForm)]
pub struct AdminLogin {
    username: String,
    password: String,
}

#[derive(FromForm)]
pub struct EditPost {
    title: String,
    content: String,
    image_url: Option<String>,
}

fn hashed_username_password() -> String {
    let admin_username = env::var("ADMIN_USERNAME").unwrap_or_default();
    let admin_password = env::var("ADMIN_PASSWORD").unwrap_or_default();

    let combined = format!("{}:{}", admin_username, admin_password);

    let mut hasher = Sha256::new();

    // Update the hasher with the combined data
    hasher.update(combined.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

fn is_admin(cookies: &CookieJar<'_>) -> bool {
    cookies
        .get("admin")
        .map(|cookie| cookie.value() == hashed_username_password())
        .unwrap_or(false)
}

#[get("/admin")]
pub async fn admin_panel(
    cookies: &CookieJar<'_>,
    db: &State<PgPool>,
) -> Result<Template, Redirect> {
    if !is_admin(cookies) {
        return Err(Redirect::to(uri!(admin_login)));
    }

    match Post::get_all(db.inner()).await {
        Ok(posts) => Ok(Template::render("admin_panel", context! { posts: posts })),
        Err(_) => Ok(Template::render(
            "admin_panel",
            context! { error: "Failed to load posts" },
        )),
    }
}

#[get("/admin/login")]
pub async fn admin_login(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    if is_admin(cookies) {
        return Err(Redirect::to(uri!(admin_panel)));
    }
    Ok(Template::render("admin_login", context! {}))
}

#[post("/admin/login", data = "<login>")]
pub async fn admin_login_post(
    cookies: &CookieJar<'_>,
    login: Form<AdminLogin>,
) -> Result<Redirect, Template> {
    let admin_username = env::var("ADMIN_USERNAME").unwrap_or_default();
    let admin_password = env::var("ADMIN_PASSWORD").unwrap_or_default();

    if login.username == admin_username && login.password == admin_password {
        let mut cookie = Cookie::build("admin").http_only(true).secure(true).finish();
        cookie.set_value(hashed_username_password());
        cookies.add(cookie);
        Ok(Redirect::to(uri!(admin_panel)))
    } else {
        Err(Template::render(
            "admin_login",
            context! { error: "Invalid credentials" },
        ))
    }
}

#[post("/admin/logout")]
pub async fn admin_logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove(Cookie::build("admin"));
    Redirect::to(uri!(admin_login))
}

#[post("/admin/posts/<id>/delete")]
pub async fn delete_post(
    cookies: &CookieJar<'_>,
    db: &State<PgPool>,
    id: i32,
) -> Result<Redirect, Template> {
    if !is_admin(cookies) {
        return Err(Template::render(
            "admin_login",
            context! { error: "Not authorized" },
        ));
    }

    match sqlx::query!("DELETE FROM posts WHERE id = $1", id)
        .execute(db.inner())
        .await
    {
        Ok(_) => Ok(Redirect::to(uri!(admin_panel))),
        Err(_) => Ok(Redirect::to(uri!(admin_panel))),
    }
}

#[post("/admin/posts/<id>/edit", data = "<post>")]
pub async fn edit_post(
    cookies: &CookieJar<'_>,
    db: &State<PgPool>,
    id: i32,
    post: Form<EditPost>,
) -> Result<Redirect, Template> {
    if !is_admin(cookies) {
        return Err(Template::render(
            "admin_login",
            context! { error: "Not authorized" },
        ));
    }

    match sqlx::query!(
        "UPDATE posts SET title = $1, content = $2, image_url = $3 WHERE id = $4",
        post.title,
        post.content,
        post.image_url,
        id
    )
    .execute(db.inner())
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(admin_panel))),
        Err(_) => Ok(Redirect::to(uri!(admin_panel))),
    }
}
