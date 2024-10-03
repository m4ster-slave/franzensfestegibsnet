mod index;


pub fn routes() -> Vec<rocket::Route> {
    routes![
        index::root,
    ]
}
