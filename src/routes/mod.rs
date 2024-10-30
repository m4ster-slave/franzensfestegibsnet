mod forum;
mod index;
mod wiki;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index::root,
        forum::forum,
        wiki::wiki,
        wiki::get_wiki_article,
    ]
}
