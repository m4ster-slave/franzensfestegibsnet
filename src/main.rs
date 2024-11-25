#[macro_use]
extern crate rocket;

mod models;
mod routes;

use rocket::fs::{relative, FileServer, Options};
use rocket_dyn_templates::Template;

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes::routes())
        .mount(
            "/public",
            FileServer::new(
                relative!("/public"),
                Options::Missing | Options::NormalizeDirs,
            ),
        )
}
