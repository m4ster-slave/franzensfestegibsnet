use rocket_dyn_templates::{context, Template};

#[get("/wiki")]
pub async fn wiki() -> Template {
    Template::render("wiki", context! {})
}
