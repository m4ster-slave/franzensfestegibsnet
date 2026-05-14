use crate::markdown::render_markdown;
use crate::models::article::{Article, ArticleView};
use crate::models::auth::CurrentUser;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use sqlx::PgPool;

#[derive(serde::Serialize)]
struct WikiArticleLink {
    slug: String,
    title: String,
    active: bool,
}

#[get("/wiki")]
pub async fn wiki(db: &State<PgPool>, current_user: Option<CurrentUser>) -> Template {
    match Article::published(db.inner()).await {
        Ok(articles) => Template::render(
            "wiki",
            context! {
                articles: wiki_nav(articles, None),
                page_title: "Wiki",
                is_welcome: true,
                current_user: current_user.map(|user| user.0),
            },
        ),
        Err(_) => Template::render(
            "wiki",
            context! {
                error: "Failed to load articles",
                page_title: "Wiki",
                is_welcome: true,
                current_user: current_user.map(|user| user.0),
            },
        ),
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

    let articles = Article::published(db.inner())
        .await
        .map_err(|_| Redirect::to("/404"))?;

    match Article::get_published_by_slug(db.inner(), slug).await {
        Ok(Some(article)) => {
            let page_title = article.title.clone();
            let view = ArticleView {
                content_html: render_markdown(&article.content),
                article,
            };
            Ok(Template::render(
                "wiki",
                context! {
                    articles: wiki_nav(articles, Some(slug)),
                    selected_article: view,
                    selected_slug: slug,
                    page_title: page_title,
                    current_user: current_user.map(|user| user.0),
                },
            ))
        }
        _ => Err(Redirect::to("/404")),
    }
}

fn wiki_nav(articles: Vec<Article>, selected_slug: Option<&str>) -> Vec<WikiArticleLink> {
    articles
        .into_iter()
        .map(|article| WikiArticleLink {
            active: selected_slug == Some(article.slug.as_str()),
            slug: article.slug,
            title: article.title,
        })
        .collect()
}
