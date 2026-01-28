use maud::{Markup, PreEscaped, html};
use crate::models::{Comment, Recipe, RecipeDetail};

const CSS: &str = r#"
body {
  font-family: system-ui, sans-serif;
  font-size: 16px;
  line-height: 1.6;
  color: #222;
  background: #fff;
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
}

a { color: #5a7d5a; text-decoration: none; }
a:hover { text-decoration: underline; }

header { margin-bottom: 30px; display: flex; justify-content: space-between; align-items: baseline; }
.logo { font-weight: 600; font-size: 18px; color: #222; text-decoration: none; }
.logo:hover { text-decoration: none; }

.recipe-item { margin-bottom: 20px; }
.recipe-title { font-size: 17px; }
.recipe-meta { font-size: 14px; color: #666; margin-top: 2px; }

h1 { font-size: 24px; font-weight: 600; margin-bottom: 5px; }
.meta { font-size: 14px; color: #666; margin-bottom: 20px; }
.description { margin-bottom: 20px; }
.info { font-size: 14px; color: #666; margin-bottom: 25px; }

h2 { font-size: 16px; font-weight: 600; margin: 25px 0 10px; }
ul, ol { margin-left: 20px; }
li { margin-bottom: 8px; }

.comments { margin-top: 40px; border-top: 1px solid #eee; padding-top: 20px; }
.comment { margin-bottom: 15px; }
.comment-meta { font-size: 13px; color: #666; }
.comment-text { margin-top: 3px; }
.comment-children { margin-left: 25px; margin-top: 10px; }
"#;

pub fn base_layout(title: &str, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) }
                style { (PreEscaped(CSS)) }
            }
            body {
                header {
                    a class="logo" href="/" { "AtChef" }
                    a href="/" { "new" }
                }
                (content)
            }
        }
    }
}

pub fn recipe_list(recipes: &[Recipe]) -> Markup {
    html! {
        @for recipe in recipes {
            div class="recipe-item" {
                div class="recipe-title" {
                    a href=(format!("/recipe/{}", recipe.id)) { (&recipe.title) }
                }
                div class="recipe-meta" {
                    "by " (&recipe.author_handle) " · " (&recipe.time_ago) " · "
                    (recipe.comment_count) " comments"
                }
            }
        }
    }
}

pub fn recipe_page(recipe: &RecipeDetail) -> Markup {
    html! {
        h1 { (&recipe.title) }
        div class="meta" {
            "by " (&recipe.author_handle) " · " (&recipe.time_ago)
        }

        p class="description" { (&recipe.description) }

        div class="info" {
            "Prep: " (&recipe.prep_time) " · Cook: " (&recipe.cook_time) " · Serves: " (recipe.servings)
        }

        h2 { "Ingredients" }
        ul {
            @for ingredient in &recipe.ingredients {
                li { (ingredient) }
            }
        }

        h2 { "Steps" }
        ol {
            @for step in &recipe.steps {
                li { (step) }
            }
        }

        @if !recipe.comments.is_empty() {
            div class="comments" {
                h2 { "Comments (" (recipe.comments.len()) ")" }
                (render_comments(&recipe.comments))
            }
        }
    }
}

fn render_comments(comments: &[Comment]) -> Markup {
    html! {
        @for comment in comments {
            div class="comment" {
                div class="comment-meta" {
                    (&comment.author_handle) " · " (&comment.time_ago)
                }
                div class="comment-text" { (&comment.text) }
                @if !comment.children.is_empty() {
                    div class="comment-children" {
                        (render_comments(&comment.children))
                    }
                }
            }
        }
    }
}
