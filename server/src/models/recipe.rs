use crate::models::AuthorInfo;
use chrono::Utc;

pub struct Recipe {
    pub id: String,
    pub name: String,
    pub author: AuthorInfo,
    pub time_ago: String,
    pub comment_count: u32,
}

impl Recipe {
    pub fn from_db_row(row: &crate::db::RecipeRow, author: AuthorInfo) -> Self {
        let now = Utc::now();
        let duration = now.signed_duration_since(row.created_at);
        let time_ago = if duration.num_seconds() < 60 {
            "just now".to_string()
        } else if duration.num_seconds() < 3600 {
            format!("{} min ago", duration.num_minutes())
        } else if duration.num_days() < 1 {
            format!("{} hours ago", duration.num_hours())
        } else {
            format!("{} days ago", duration.num_days())
        };

        Recipe {
            id: row.rkey.clone(),
            name: row.name.clone(),
            author,
            time_ago,
            comment_count: 0,
        }
    }
}

#[allow(dead_code)]
pub struct RecipeDetail {
    pub id: String,
    pub name: String,
    pub content: String,
    pub portions: u32,
    pub time: u32,
    pub author: AuthorInfo,
    pub time_ago: String,
    pub comments: Vec<Comment>,
    pub description: Option<String>,
    pub prep_time: Option<u32>,
    pub cook_time: Option<u32>,
}

#[allow(dead_code)]
pub struct Comment {
    pub id: String,
    pub author: AuthorInfo,
    pub text: String,
    pub time_ago: String,
    pub children: Vec<Comment>,
}
