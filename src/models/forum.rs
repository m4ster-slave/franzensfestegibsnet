#[derive(FromForm)]
pub(crate) struct CreatePost {
    pub(crate) title: String,
    pub(crate) content: String,
}
