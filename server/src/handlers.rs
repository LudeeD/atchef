use axum::extract::Path;
use maud::Markup;

use crate::models::{get_mock_recipe_detail, get_mock_recipes};
use crate::views::{base_layout, recipe_list, recipe_page};

pub async fn home() -> Markup {
    let recipes = get_mock_recipes();
    let content = recipe_list(&recipes);
    base_layout("AtChef", content)
}

pub async fn recipe(Path(id): Path<String>) -> Markup {
    match get_mock_recipe_detail(&id) {
        Some(recipe) => {
            let content = recipe_page(&recipe);
            base_layout(&format!("{} | AtChef", recipe.title), content)
        }
        None => base_layout("Not Found | AtChef", maud::html! {
            h1 { "Recipe not found" }
            p { "The recipe you're looking for doesn't exist." }
            p { a href="/" { "Back to home" } }
        }),
    }
}
