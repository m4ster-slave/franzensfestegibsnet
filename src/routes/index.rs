use rocket_dyn_templates::{context, Template};

#[get("/")]
pub async fn root() -> Template {
    Template::render("index", context! {})
}
