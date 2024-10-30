use pulldown_cmark::{html, Parser};
use rocket_dyn_templates::{context, Template};
use std::collections::HashMap;
use std::{fs, io};

use rocket::response::Redirect;
use std::path::Path;

fn get_filenames_from_directory(dir_path: &str) -> io::Result<Vec<String>> {
    let mut filenames = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        // Check if the entry is a file
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                // Check if the filename ends with ".md" and trim the extension
                if filename.ends_with(".md") {
                    let trimmed_filename = filename.trim_end_matches(".md");
                    filenames.push(trimmed_filename.to_string());
                }
            }
        }
    }

    Ok(filenames)
}

#[get("/wiki")]
pub async fn wiki() -> Template {
    let md_files = get_filenames_from_directory("./articles/").unwrap();

    Template::render("wiki", context! {md_files})
}

#[get("/wiki/<wiki_article>")]
pub async fn get_wiki_article(wiki_article: &str) -> Result<Template, Redirect> {
    // Define allowed characters to prevent directory traversal
    if !wiki_article
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
    {
        return Err(Redirect::to("/404"));
    }

    // get the full path of the base dir
    let base_dir = Path::new("./articles").canonicalize().unwrap();
    let wiki_article = format!("{}.md", wiki_article);

    // Resolve the path and ensure it stays within the base directory
    let article_path = base_dir.join(wiki_article);
    let article_path = match article_path.canonicalize() {
        Ok(path) if path.starts_with(base_dir) => path,
        _ => return Err(Redirect::to("/404")),
    };

    let markdown_input = match fs::read_to_string(article_path) {
        Ok(content) => content,
        Err(_) => return Err(Redirect::to("/404")),
    };

    // Parse the Markdown content to HTML
    let mut html_output = String::new();
    let parser = Parser::new(&markdown_input);
    html::push_html(&mut html_output, parser);

    // Pass the HTML to the template
    let mut context = HashMap::new();
    context.insert("content", html_output);

    Ok(Template::render("wiki_article", &context))
}
