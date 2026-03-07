#[derive(Clone, Debug)]
pub struct AuthorInfo {
    pub handle: String,
}

impl AuthorInfo {
    /// Create a basic AuthorInfo from just handle
    pub fn basic(handle: String) -> Self {
        Self { handle }
    }
}
