use crate::markdown::render_markdown;
use crate::models::auth::{CurrentUser, User};
use crate::models::forum::*;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sqlx::PgPool;

#[get("/forum?<page>")]
pub async fn forum(
    db: &State<PgPool>,
    page: Option<i64>,
    current_user: Option<CurrentUser>,
) -> Template {
    let mut page = page.unwrap_or(1);
    let items_per_page = 10;

    if page < 1 {
        page = 1;
    }

    match Post::get_paginated(db.inner(), page, items_per_page).await {
        Ok((posts, pagination)) => {
            let author_ids: Vec<i32> = posts.iter().filter_map(|post| post.author_id).collect();
            let authors = User::summaries_by_ids(db.inner(), &author_ids)
                .await
                .unwrap_or_default();
            let posts: Vec<PostListItem> = posts
                .into_iter()
                .map(|post| PostListItem {
                    author: post.author_id.and_then(|id| authors.get(&id).cloned()),
                    post,
                })
                .collect();

            Template::render(
                "forum",
                context! {
                    posts: posts,
                    pagination: pagination,
                    current_user: current_user.map(|user| user.0),
                },
            )
        }
        Err(_) => Template::render("forum", context! { error: "Failed to load posts" }),
    }
}

#[get("/forum/<id>?<page>")]
pub async fn view_post(
    db: &State<PgPool>,
    id: i32,
    page: Option<i64>,
    current_user: Option<CurrentUser>,
) -> Template {
    let mut page = page.unwrap_or(1);
    let items_per_page = 10;

    if page < 1 {
        page = 1;
    }

    let post_result = Post::get_visible_by_id(db.inner(), id).await;
    let comments_result =
        Comment::get_paginated_by_post_id(db.inner(), id, page, items_per_page).await;

    match (post_result, comments_result) {
        (Ok(Some(post)), Ok((comments, pagination))) => {
            let mut author_ids: Vec<i32> = post.author_id.into_iter().collect();
            author_ids.extend(comments.iter().filter_map(|comment| comment.author_id));
            let authors = User::summaries_by_ids(db.inner(), &author_ids)
                .await
                .unwrap_or_default();

            let rendered_post = RenderedPost {
                author: post.author_id.and_then(|id| authors.get(&id).cloned()),
                content_html: render_markdown(&post.content),
                post,
            };
            let rendered_comments: Vec<RenderedComment> = comments
                .into_iter()
                .map(|comment| RenderedComment {
                    author: comment.author_id.and_then(|id| authors.get(&id).cloned()),
                    content_html: render_markdown(&comment.content),
                    comment,
                })
                .collect();

            Template::render(
                "forum_post",
                context! {
                    rendered_post: rendered_post,
                    comments: rendered_comments,
                    pagination: pagination,
                    current_user: current_user.map(|user| user.0),
                },
            )
        }
        _ => Template::render("forum", context! { error: "Failed to load post" }),
    }
}

#[post("/forum/create", data = "<post>")]
pub async fn create_post(
    db: &State<PgPool>,
    post: Form<CreatePostFingerprint>,
    client_info: ClientInfo,
    current_user: Option<CurrentUser>,
) -> Result<Redirect, Template> {
    let Some(current_user) = current_user else {
        return Ok(Redirect::to("/login"));
    };

    let post = post.into_inner();
    let client_info = if client_info.fingerprint.is_empty() && !post.fingerprint.is_empty() {
        ClientInfo {
            fingerprint: post.fingerprint.clone(),
            ..client_info
        }
    } else {
        client_info
    };

    match Post::validate_client(db.inner(), &client_info).await {
        Ok(true) => {
            let create_result = Post::create(
                db.inner(),
                CreatePost {
                    title: post.title,
                    content: post.content,
                },
                current_user.0.id,
            )
            .await;

            match create_result {
                Ok(_) => {
                    if (Post::update_client_info(db.inner(), &client_info).await).is_err() {
                        return Err(Template::render(
                            "forum_create",
                            context! { error: "Failed to update client info" },
                        ));
                    }
                    Ok(Redirect::to(uri!(forum(page = None::<i64>))))
                }
                Err(_) => Err(Template::render(
                    "forum_create",
                    context! { error: "Failed to create post" },
                )),
            }
        }
        Ok(false) => Err(Template::render(
            "forum_create",
            context! { error: "Too many posts. Please wait before posting again." },
        )),
        Err(_) => Err(Template::render(
            "forum_create",
            context! { error: "Failed to validate post" },
        )),
    }
}

#[get("/forum/create")]
pub async fn forum_create(current_user: Option<CurrentUser>) -> Result<Template, Redirect> {
    let Some(current_user) = current_user else {
        return Err(Redirect::to("/login"));
    };

    Ok(Template::render(
        "forum_create",
        context! { current_user: current_user.0 },
    ))
}

#[post("/forum/<post_id>/comment", data = "<comment>")]
pub async fn create_comment(
    db: &State<PgPool>,
    post_id: i32,
    comment: Form<CreateComment>,
    current_user: Option<CurrentUser>,
) -> Result<Redirect, Template> {
    let Some(current_user) = current_user else {
        return Ok(Redirect::to("/login"));
    };

    let post = match Post::get_visible_by_id(db.inner(), post_id).await {
        Ok(Some(post)) => post,
        _ => {
            return Err(Template::render(
                "forum",
                context! { error: "Post not found" },
            ))
        }
    };

    if post.locked {
        return Err(Template::render(
            "forum_post",
            context! { error: "This post is locked", rendered_post: RenderedPost { author: None, content_html: render_markdown(&post.content), post } },
        ));
    }

    match Comment::create(db.inner(), post_id, comment.into_inner(), current_user.0.id).await {
        Ok(_) => Ok(Redirect::to(uri!(view_post(
            id = post_id,
            page = Option::<i64>::None
        )))),
        Err(_) => Err(Template::render(
            "forum_post",
            context! { error: "Failed to add comment" },
        )),
    }
}
