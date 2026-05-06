use dotenv::dotenv;
use sqlx::PgPool;
use std::{env, fs, path::Path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")?;
    let articles_dir = env::args()
        .nth(1)
        .unwrap_or_else(|| "./articles".to_string());

    let pool = PgPool::connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let mut imported = 0;
    for entry in fs::read_dir(&articles_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let title = title_from_markdown(&content).unwrap_or_else(|| title_from_filename(&path));
        let slug = slugify(&title);

        let result = sqlx::query(
            r#"
            INSERT INTO articles (slug, title, content, status)
            VALUES ($1, $2, $3, 'published')
            ON CONFLICT (slug) DO NOTHING
            "#,
        )
        .bind(&slug)
        .bind(&title)
        .bind(&content)
        .execute(&pool)
        .await?;

        if result.rows_affected() > 0 {
            imported += 1;
            println!("Imported {} as /wiki/{}", title, slug);
        }
    }

    println!("Imported {imported} new article(s).");
    Ok(())
}

fn title_from_markdown(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("# ")
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn title_from_filename(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled Article")
        .replace(['_', '-'], " ")
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}
