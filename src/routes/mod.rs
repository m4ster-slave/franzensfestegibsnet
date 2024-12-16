mod admin;
mod forum;
mod index;
mod wiki;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index::root,
        forum::forum,
        forum::create_post,
        forum::view_post,
        forum::create_comment,
        forum::forum_create,
        wiki::wiki,
        wiki::get_wiki_article,
        admin::admin_panel,
        admin::admin_login,
        admin::admin_login_post,
        admin::admin_logout,
        admin::edit_post,
        admin::delete_post,
    ]
}
