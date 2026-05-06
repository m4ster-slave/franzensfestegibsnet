use crate::markdown::render_markdown;
use crate::models::article::{Article, ArticleView};
use crate::models::auth::CurrentUser;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sqlx::PgPool;

#[get("/wiki")]
pub async fn wiki(db: &State<PgPool>, current_user: Option<CurrentUser>) -> Template {
    match Article::published(db.inner()).await {
        Ok(articles) => Template::render(
            "wiki",
            context! {
                articles: articles,
                current_user: current_user.map(|user| user.0),
            },
        ),
        Err(_) => Template::render("wiki", context! { error: "Failed to load articles" }),
    }
}

#[get("/wiki/<slug>")]
pub async fn get_wiki_article(
    db: &State<PgPool>,
    slug: &str,
    current_user: Option<CurrentUser>,
) -> Result<Template, Redirect> {
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(Redirect::to("/404"));
    }

    match Article::get_published_by_slug(db.inner(), slug).await {
        Ok(Some(article)) => {
            let view = ArticleView {
                content_html: render_markdown(&article.content),
                article,
            };
            Ok(Template::render(
                "wiki_article",
                context! {
                    view: view,
                    current_user: current_user.map(|user| user.0),
                },
            ))
        }
        _ => Err(Redirect::to("/404")),
    }
}
