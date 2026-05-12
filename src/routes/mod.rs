mod admin;
pub mod auth;
mod forum;
mod index;
mod uploads;
mod wiki;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index::root,
        index::robots,
        index::sitemap,
        auth::register,
        auth::register_post,
        auth::login,
        auth::login_post,
        auth::profile,
        auth::logout,
        forum::forum,
        forum::create_post,
        forum::view_post,
        forum::create_comment,
        forum::forum_create,
        wiki::wiki,
        wiki::get_wiki_article,
        admin::admin_panel,
        admin::users_panel,
        admin::set_user_role,
        admin::disable_user,
        admin::enable_user,
        admin::reset_user_password,
        admin::articles_panel,
        admin::new_article,
        admin::create_article,
        admin::edit_article,
        admin::update_article,
        admin::delete_article,
        admin::moderation_panel,
        admin::moderation_posts,
        admin::moderation_comments,
        admin::moderate_post,
        admin::lock_post,
        admin::unlock_post,
        admin::edit_post_panel,
        admin::delete_post,
        admin::update_post,
        admin::moderate_comment,
        admin::delete_comment,
        admin::edit_comment,
        uploads::upload_image,
        uploads::update_profile_avatar,
        uploads::remove_profile_avatar,
    ]
}
