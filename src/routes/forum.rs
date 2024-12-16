use crate::models::forum::*;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sqlx::PgPool;

#[get("/forum")]
pub async fn forum(db: &State<PgPool>) -> Template {
    match Post::get_all(db.inner()).await {
        Ok(posts) => Template::render("forum", context! { posts: posts }),
        Err(_) => Template::render("forum", context! { error: "Failed to load posts" }),
    }
}

#[get("/forum/<id>")]
pub async fn view_post(db: &State<PgPool>, id: i32) -> Template {
    let post_result = Post::get_by_id(db.inner(), id).await;
    let comments_result = Comment::get_by_post_id(db.inner(), id).await;

    match (post_result, comments_result) {
        (Ok(post), Ok(comments)) => {
            Template::render("forum_post", context! { post: post, comments: comments })
        }
        _ => Template::render("forum", context! { error: "Failed to load post" }),
    }
}

#[post("/forum/create", data = "<post>")]
pub async fn create_post(
    db: &State<PgPool>,
    post: Form<CreatePostFingerprint>,
) -> Result<Redirect, Template> {
    println!("User Fingerprint: {}", post.fingerprint);

    match Post::create(
        db.inner(),
        CreatePost {
            title: post.title.clone(),
            content: post.content.clone(),
            image_url: post.image_url.clone(),
        },
    )
    .await
    {
        Ok(_) => Ok(Redirect::to(uri!(forum))),
        Err(_) => Err(Template::render(
            "forum_create",
            context! {
                error: "Failed to create post"
            },
        )),
    }
}

#[get("/forum/create")]
pub async fn forum_create() -> Template {
    Template::render("forum_create", context! {})
}

#[post("/forum/<post_id>/comment", data = "<comment>")]
pub async fn create_comment(
    db: &State<PgPool>,
    post_id: i32,
    comment: Form<CreateComment>,
) -> Result<Redirect, Template> {
    match Comment::create(db.inner(), post_id, comment.into_inner()).await {
        Ok(_) => Ok(Redirect::to(uri!(view_post(post_id)))),
        Err(_) => Err(Template::render(
            "forum_post",
            context! {
                error: "Failed to add comment",
                post_id: post_id,
            },
        )),
    }
}
