mod index;
mod wiki;
mod forum;


pub fn routes() -> Vec<rocket::Route> {
    routes![
        index::root,
        forum::forum,
        wiki::wiki,
    ]
}
