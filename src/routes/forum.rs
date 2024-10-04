use rocket_dyn_templates::{context, Template};

#[get("/forum")]
pub async fn forum() -> Template {
    Template::render("forum", context! {})
}
