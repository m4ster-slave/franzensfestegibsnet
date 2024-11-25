use crate::models;
use rocket::form::Form;
use rocket_dyn_templates::{context, Template};

#[get("/forum")]
pub async fn forum() -> Template {
    Template::render("forum", context! {})
}

#[get("/forum/create")]
pub async fn create_page() -> Template {
    Template::render("forum_create", context! {})
}

#[post("/forum/create", data = "<post>")]
pub async fn create_post(post: Form<models::forum::CreatePost>) -> Template {
    println!("title: \"{}\", content: \n{}\n", post.title, post.content);
    Template::render("forum_create", context! {})
}
