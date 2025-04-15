use crate::models::forum::*;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sqlx::PgPool;

#[get("/forum?<page>")]
pub async fn forum(db: &State<PgPool>, page: Option<i64>) -> Template {
    let mut page = page.unwrap_or(1);
    let items_per_page = 10;

    if page < 1 {
        page = 1;
    }

    match Post::get_paginated(db.inner(), page, items_per_page).await {
        Ok((posts, pagination)) => Template::render(
            "forum",
            context! {
                posts: posts,
                pagination: pagination,
            },
        ),
        Err(_) => Template::render("forum", context! { error: "Failed to load posts" }),
    }
}

#[get("/forum/<id>?<page>")]
pub async fn view_post(db: &State<PgPool>, id: i32, page: Option<i64>) -> Template {
    let mut page = page.unwrap_or(1);
    let items_per_page = 10;

    if page < 1 {
        page = 1;
    }

    let post_result = Post::get_by_id(db.inner(), id).await;
    let comments_result =
        Comment::get_paginated_by_post_id(db.inner(), id, page, items_per_page).await;

    match (post_result, comments_result) {
        (Ok(post), Ok((comments, pagination))) => Template::render(
            "forum_post",
            context! {
                post: post,
                comments: comments,
                pagination: pagination,
            },
        ),
        _ => Template::render("forum", context! { error: "Failed to load post" }),
    }
}

#[post("/forum/create", data = "<post>")]
pub async fn create_post(
    db: &State<PgPool>,
    post: Form<CreatePostFingerprint>,
    client_info: ClientInfo,
) -> Result<Redirect, Template> {
    match Post::validate_client(db.inner(), &client_info).await {
        Ok(true) => {
            let create_result = Post::create(
                db.inner(),
                CreatePost {
                    title: post.title.clone(),
                    content: post.content.clone(),
                    image_url: post.image_url.clone(),
                },
            )
            .await;

            match create_result {
                Ok(_) => {
                    if (Post::update_client_info(db.inner(), &client_info).await).is_err() {
                        return Err(Template::render(
                            "forum_create",
                            context! {
                                error: "Failed to update client info"
                            },
                        ));
                    }
                    Ok(Redirect::to(uri!(forum(page = None::<i64>))))
                }
                Err(_) => Err(Template::render(
                    "forum_create",
                    context! {
                        error: "Failed to create post"
                    },
                )),
            }
        }
        Ok(false) => Err(Template::render(
            "forum_create",
            context! {
                error: "Too many posts. Please wait before posting again."
            },
        )),
        Err(_) => Err(Template::render(
            "forum_create",
            context! {
                error: "Failed to validate post"
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
        Ok(_) => Ok(Redirect::to(uri!(view_post(
            id = post_id,
            page = Option::<i64>::None
        )))),
        Err(_) => Err(Template::render(
            "forum_post",
            context! {
                error: "Failed to add comment",
                post_id: post_id,
            },
        )),
    }
}
