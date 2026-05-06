#[macro_use]
extern crate rocket;

mod helpers;
mod markdown;
mod models;
mod routes;

use dotenv::dotenv;
use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
use sqlx::PgPool;
use std::env;
use tokio::time::{sleep, Duration};

#[launch]
async fn rocket() -> _ {
    eprintln!("Starting Server....");

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = connect_with_retry(&database_url).await;

    eprintln!("Connected to server....");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    models::auth::bootstrap_admin(&pool)
        .await
        .expect("Failed to bootstrap admin user");

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .expect("Failed to create upload directory");

    let template = Template::custom(|engines| {
        helpers::register_helpers(&mut engines.handlebars);
    });

    eprintln!("Templates registered.....");

    rocket::build()
        .manage(pool)
        .attach(template)
        .mount("/", routes::routes())
        .mount("/uploads", FileServer::from(upload_dir))
        .mount("/css", FileServer::from(relative!("/public/css")))
        .mount("/js", FileServer::from(relative!("/public/js")))
        .mount("/static", FileServer::from(relative!("/public/static")))
}

async fn connect_with_retry(database_url: &str) -> PgPool {
    let mut attempts = 0;

    loop {
        match PgPool::connect(database_url).await {
            Ok(pool) => return pool,
            Err(error) if attempts < 20 => {
                attempts += 1;
                eprintln!("Postgres connection failed ({error}); retrying...");
                sleep(Duration::from_secs(2)).await;
            }
            Err(error) => panic!("Failed to connect to Postgres: {error}"),
        }
    }
}
