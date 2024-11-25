mod forum;
mod index;
mod wiki;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index::root,
        forum::forum,
        forum::create_page,
        forum::create_post,
        wiki::wiki,
        wiki::get_wiki_article,
    ]
}
