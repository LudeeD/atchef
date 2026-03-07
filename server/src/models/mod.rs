pub mod author;
pub mod recipe;
pub mod user;

// Re-export all public types for convenience
pub use author::AuthorInfo;
pub use recipe::{Comment, Recipe, RecipeDetail};
pub use user::ProfileRecord;
