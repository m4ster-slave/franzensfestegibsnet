#[macro_use]
extern crate rocket;

mod helpers;
mod models;
mod routes;

use dotenv::dotenv;
use rocket::fs::{relative, FileServer, Options};
use rocket_dyn_templates::Template;
use sqlx::PgPool;
use std::env;

#[launch]
async fn rocket() -> _ {
    eprintln!("Starting Server....");

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    eprintln!("Connected to server....");

    let template = Template::custom(|engines| {
        helpers::register_helpers(&mut engines.handlebars);
    });

    eprintln!("Templates registered.....");

    rocket::build()
        .manage(pool)
        .attach(template)
        .mount("/", routes::routes())
        .mount(
            "/public",
            FileServer::new(
                relative!("/public"),
                Options::Missing | Options::NormalizeDirs,
            ),
        )
}
