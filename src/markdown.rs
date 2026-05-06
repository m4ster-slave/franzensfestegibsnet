use ammonia::Builder;
use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;

pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut skipping_external_image = false;
    let parser = Parser::new_ext(markdown, options).filter_map(|event| match event {
        Event::Start(Tag::Image { dest_url, .. }) if !is_local_upload(&dest_url) => {
            skipping_external_image = true;
            None
        }
        Event::End(TagEnd::Image) if skipping_external_image => {
            skipping_external_image = false;
            None
        }
        Event::Text(_) if skipping_external_image => None,
        other if skipping_external_image => match other {
            Event::End(TagEnd::Image) => {
                skipping_external_image = false;
                None
            }
            _ => None,
        },
        other => Some(other),
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    sanitize_html(&html_output)
}

fn is_local_upload(src: &str) -> bool {
    src.starts_with("/uploads/")
        && !src.contains("..")
        && !src.contains('\\')
        && !src.contains("//")
}

fn sanitize_html(html: &str) -> String {
    let tags: HashSet<&str> = [
        "a",
        "blockquote",
        "br",
        "code",
        "del",
        "em",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "hr",
        "img",
        "li",
        "ol",
        "p",
        "pre",
        "strong",
        "table",
        "tbody",
        "td",
        "th",
        "thead",
        "tr",
        "ul",
    ]
    .into_iter()
    .collect();

    let generic_attributes: HashSet<&str> = ["class"].into_iter().collect();
    let img_attributes: HashSet<&str> = ["src", "alt", "title"].into_iter().collect();
    let a_attributes: HashSet<&str> = ["href", "title"].into_iter().collect();

    Builder::new()
        .tags(tags)
        .generic_attributes(generic_attributes)
        .tag_attributes(
            [("img", img_attributes), ("a", a_attributes)]
                .into_iter()
                .collect(),
        )
        .clean(html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn keeps_local_upload_images() {
        let html = render_markdown("![x](/uploads/example.png)");
        assert!(html.contains("<img"));
        assert!(html.contains("/uploads/example.png"));
    }

    #[test]
    fn drops_external_images() {
        let html = render_markdown("![x](https://example.com/image.png)");
        assert!(!html.contains("<img"));
        assert!(!html.contains("https://example.com/image.png"));
    }

    #[test]
    fn strips_script_html() {
        let html = render_markdown("<script>alert(1)</script>\n\n**ok**");
        assert!(!html.contains("<script>"));
        assert!(html.contains("<strong>ok</strong>"));
    }
}
