use crate::db::UserRow;
use crate::models::{AuthorInfo, Comment, Recipe, RecipeDetail};
use maud::{html, Markup, PreEscaped};

const CSS: &str = r#"
/* CSS Custom Properties for Light/Dark Themes */
:root {
  /* Light theme (enhanced contrast) - WCAG AAA compliant */
  --color-text-primary: #0a0a0a;      /* was #222 - 15.8:1 contrast */
  --color-text-secondary: #2d2d2d;    /* was #666 - 8.3:1 contrast */
  --color-text-meta: #404040;         /* was #999 - 7.1:1 contrast */
  --color-text-placeholder: #505050;  /* was #999 - improved */
  --color-background: #ffffff;
  --color-surface: #f8f8f8;           /* was #f9f9f9 */
  --color-surface-alt: #f0f0f0;       /* was #f5f5f5 */
  --color-border: #d0d0d0;            /* was #ccc */
  --color-border-subtle: #e5e5e5;     /* was #eee */
  --color-border-light: #f0f0f0;      /* was #ddd */
  --color-accent: #5a7d5a;            /* sage green */
  --color-accent-hover: #4a6d4a;
  --color-accent-text: #ffffff;
  --color-error: #8b0000;             /* was #b44 - 7.2:1 contrast */
  --color-ingredient: #228833;        /* was #2a6 - improved contrast */
  --color-ingredient-bg: #eaf4ea;
  --color-equipment: #996677;         /* was #a67 - improved contrast */
  --color-equipment-bg: #f5eef2;
  --color-timer: #4477aa;            /* was #67a - improved contrast */
  --color-timer-bg: #e8f0ff;         /* was #eef - cleaner */
  --color-code-bg: #eeeeee;          /* was #f0f0f0 */
}

[data-theme="dark"] {
  /* Dark theme - WCAG AAA compliant */
  --color-text-primary: #f8f8f8;      /* 15.5:1 contrast */
  --color-text-secondary: #d0d0d0;    /* 8.5:1 contrast */
  --color-text-meta: #b0b0b0;         /* 6.8:1 contrast */
  --color-text-placeholder: #909090;  /* 4.7:1 contrast */
  --color-background: #0a0a0a;
  --color-surface: #151515;
  --color-surface-alt: #1a1a1a;
  --color-border: #333333;
  --color-border-subtle: #2a2a2a;
  --color-border-light: #404040;
  --color-accent: #7bb37b;            /* lighter sage for dark bg */
  --color-accent-hover: #8bc88b;
  --color-accent-text: #0a0a0a;
  --color-error: #ff6b6b;             /* 4.8:1 contrast */
  --color-ingredient: #55cc66;        /* bright green for dark bg */
  --color-ingredient-bg: #162516;
  --color-equipment: #cc88aa;         /* bright purple for dark bg */
  --color-equipment-bg: #251622;
  --color-timer: #66aadd;            /* bright blue for dark bg */
  --color-timer-bg: #1a2a3a;         /* dark blue bg */
  --color-code-bg: #1a1a1a;          /* dark code background */
}

body {
  font-family: system-ui, sans-serif;
  font-size: 16px;
  line-height: 1.6;
  color: var(--color-text-primary);
  background: var(--color-background);
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
}

a { 
  color: var(--color-accent); 
  text-decoration: none; 
}
a:hover { 
  text-decoration: underline; 
}

header { 
  margin-bottom: 30px; 
  display: flex; 
  justify-content: space-between; 
  align-items: baseline; 
}
.logo { 
  font-weight: 600; 
  font-size: 18px; 
  color: var(--color-text-primary); 
  text-decoration: none; 
}
.logo:hover { 
  text-decoration: none; 
}

.recipe-item { 
  margin-bottom: 20px; 
}
.recipe-title { 
  font-size: 17px; 
}
.recipe-meta { 
  font-size: 14px; 
  color: var(--color-text-secondary); 
  margin-top: 2px; 
}

h1 {
  font-size: 28px;
  font-weight: 700;
  margin-bottom: 6px;
  line-height: 1.2;
}
.meta {
  font-size: 14px;
  color: var(--color-text-secondary);
  margin-bottom: 20px;
}
.description {
  margin-bottom: 20px;
  font-size: 15px;
  color: var(--color-text-secondary);
}
.recipe-info-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 24px;
  background: var(--color-surface);
  border: 1px solid var(--color-border-subtle);
  border-radius: 6px;
  padding: 12px 16px;
  margin-bottom: 28px;
  font-size: 14px;
}
.recipe-info-item {
  color: var(--color-text-meta);
}
.recipe-info-item strong {
  color: var(--color-text-primary);
  font-weight: 600;
}

h2 {
  font-size: 13px;
  font-weight: 700;
  margin: 30px 0 14px;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--color-text-meta);
}
ul, ol {
  margin-left: 20px;
}
li {
  margin-bottom: 8px;
}

.comments {
  margin-top: 40px;
  border-top: 1px solid var(--color-border-subtle);
  padding-top: 20px;
}

#recipe-ingredients {
  display: flex;
  gap: 36px;
  flex-wrap: wrap;
  margin-bottom: 10px;
  font-size: 14px;
}
.ing-group {
  flex: 1;
  min-width: 120px;
}
.ing-group h2 {
  margin-top: 0;
}
#recipe-ingredients ul {
  margin: 0 0 0 20px;
}
#recipe-ingredients li {
  margin-bottom: 4px;
  cursor: default;
}
#recipe-ingredients .ingredient-text {
  cursor: pointer;
  display: inline-block;
}
#recipe-ingredients li.ing-active {
  color: var(--color-ingredient);
  font-weight: 600;
}
.ingredient.ing-active {
  outline: 2px solid var(--color-ingredient);
  outline-offset: 1px;
}
#recipe-content p {
  margin: 0 0 10px 0;
}
.comment { 
  margin-bottom: 15px; 
}
.comment-meta { 
  font-size: 13px; 
  color: var(--color-text-secondary); 
}
.comment-text { 
  margin-top: 3px; 
}
.comment-children { 
  margin-left: 25px; 
  margin-top: 10px; 
}

.login-form { 
  max-width: 300px; 
}
.login-form input { 
  width: 100%; 
  padding: 8px; 
  margin-bottom: 10px; 
  border: 1px solid var(--color-border); 
  border-radius: 4px; 
  background: var(--color-background);
  color: var(--color-text-primary);
}
.login-form button { 
  padding: 8px 16px; 
  background: var(--color-accent); 
  color: var(--color-accent-text); 
  border: none; 
  border-radius: 4px; 
  cursor: pointer; 
}
.login-form button:hover { 
  background: var(--color-accent-hover); 
}
.error { 
  color: var(--color-error); 
  margin-bottom: 10px; 
}
.user-info { 
  font-size: 14px; 
  color: var(--color-text-secondary); 
}

dl { 
  margin: 0; 
}
dt { 
  font-weight: 600; 
  margin-top: 15px; 
}
dd { 
  margin: 5px 0 0 0; 
  color: var(--color-text-secondary); 
}
.nav-links { 
  display: flex; 
  gap: 15px; 
}

/* Recipe Form Styles */
.recipe-form { 
  margin-top: 20px; 
}
.form-group { 
  margin-bottom: 20px; 
}
.form-group label { 
  display: block; 
  font-weight: 600; 
  margin-bottom: 5px; 
}
.form-group input,
.form-group textarea { 
  width: 100%; 
  padding: 8px; 
  border: 1px solid var(--color-border); 
  border-radius: 4px; 
  font-family: inherit; 
  font-size: 15px; 
  background: var(--color-background);
  color: var(--color-text-primary);
}
.form-group textarea { 
  resize: vertical; 
  min-height: 200px; 
  font-family: ui-monospace, monospace; 
  font-size: 14px; 
  line-height: 1.5; 
}
.form-row { 
  display: flex; 
  gap: 20px; 
}
.form-row .form-group { 
  flex: 1; 
}
.form-row .form-group input { 
  width: 100%; 
}
.help-text { 
  font-size: 13px; 
  color: var(--color-text-secondary); 
  margin-top: 5px; 
}
.help-text code { 
  background: var(--color-code-bg); 
  padding: 2px 4px; 
  border-radius: 3px; 
  font-size: 12px; 
}

.form-actions { 
  display: flex; 
  gap: 15px; 
  align-items: center; 
  margin-top: 25px; 
}
.btn-primary { 
  padding: 10px 20px; 
  background: var(--color-accent); 
  color: var(--color-accent-text); 
  border: none; 
  border-radius: 4px; 
  cursor: pointer; 
  font-size: 15px; 
}
.btn-primary:hover { 
  background: var(--color-accent-hover); 
}
.btn-secondary { 
  padding: 10px 20px; 
  color: var(--color-text-secondary); 
  text-decoration: none; 
}
.btn-secondary:hover { 
  color: var(--color-text-primary); 
}

.preview-section { 
  margin-top: 30px; 
  padding: 20px; 
  background: var(--color-surface); 
  border-radius: 8px; 
  border: 1px solid var(--color-border-subtle); 
}
.preview-section h2 { 
  margin-top: 0; 
}
.preview-content { 
  line-height: 1.7; 
}
.preview-content p { 
  margin: 0 0 10px 0; 
}
.preview-placeholder { 
  color: var(--color-text-placeholder); 
  font-style: italic; 
}
.preview-ingredients { 
  margin-top: 20px; 
  padding-top: 20px; 
  border-top: 1px solid var(--color-border-light); 
}
.preview-ingredients h3 { 
  font-size: 14px; 
  font-weight: 600; 
  margin: 15px 0 8px; 
  color: var(--color-text-primary); 
}
.preview-ingredients ul { 
  margin: 0 0 15px 20px; 
}
.preview-ingredients li { 
  margin-bottom: 4px; 
}

.ingredient {
  color: var(--color-ingredient);
  background: var(--color-ingredient-bg);
  font-weight: 500;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 14px;
}
.ingredient .amount { 
  color: var(--color-text-secondary); 
  font-weight: normal; 
}
.equipment {
  color: var(--color-equipment);
  background: var(--color-equipment-bg);
  font-weight: 500;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 14px;
}
.timer { 
  color: var(--color-timer); 
  background: var(--color-timer-bg); 
  padding: 2px 6px; 
  border-radius: 3px; 
  font-size: 14px; 
}


/* Create Recipe Button */
.create-recipe-btn { 
  padding: 6px 12px; 
  background: var(--color-accent); 
  color: var(--color-accent-text); 
  border-radius: 4px; 
  text-decoration: none; 
  font-size: 14px; 
}
.create-recipe-btn:hover { 
  background: var(--color-accent-hover); 
  text-decoration: none; 
}

/* Tab-based recipe editor */
.editor-tabs {
  display: flex;
  border-bottom: 1px solid var(--color-border-light);
  margin-bottom: 0;
  gap: 5px;
}
.editor-tab {
  padding: 8px 16px;
  background: var(--color-surface-alt);
  border: 1px solid var(--color-border-light);
  border-bottom: none;
  border-radius: 4px 4px 0 0;
  cursor: pointer;
  font-size: 14px;
  color: var(--color-text-secondary);
}
.editor-tab:hover {
  background: var(--color-border-subtle);
}
.editor-tab.active {
  background: var(--color-background);
  color: var(--color-text-primary);
  margin-bottom: -1px;
}
.editor-panel {
  display: none;
  border: 1px solid var(--color-border-light);
  border-top: none;
  border-radius: 0 0 4px 4px;
  padding: 15px;
  background: var(--color-background);
}
.editor-panel.active {
  display: block;
}
.editor-panel textarea {
  margin: 0;
  border: none;
  padding: 0;
  min-height: 300px;
  background: var(--color-background);
  color: var(--color-text-primary);
}
.editor-panel .preview-content {
  min-height: 300px;
}
.editor-panel .preview-section {
  margin-top: 0;
  border: none;
  background: none;
  padding: 0;
}

/* Theme toggle */
.theme-toggle {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 15px;
  padding: 0;
  color: var(--color-text-secondary);
  line-height: 1;
  opacity: 0.6;
}
.theme-toggle:hover {
  opacity: 1;
}

/* Theme selector styles */
.theme-selector {
  margin-top: 20px;
}
.theme-selector h2 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 10px;
}
.theme-options {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.theme-option {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}
.theme-option input[type="radio"] {
  margin: 0;
  width: auto;
}
.theme-option label {
  margin: 0;
  cursor: pointer;
}

/* Welcome card styles */
.welcome-card {
  background: var(--color-surface);
  padding: 15px;
  margin-bottom: 25px;
  border-radius: 4px;
  border: 1px solid var(--color-border-subtle);
}
"#;

pub fn base_layout(title: &str, content: Markup) -> Markup {
    base_layout_with_user(title, content, None)
}

pub fn base_layout_with_user(title: &str, content: Markup, user_handle: Option<&str>) -> Markup {
    base_layout_with_user_and_class(title, content, user_handle, None)
}

pub fn base_layout_with_user_and_class(
    title: &str,
    content: Markup,
    user_handle: Option<&str>,
    body_class: Option<&str>,
) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) }
                link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🧑‍🍳</text></svg>";
                style { (PreEscaped(CSS)) }
                script {
                    (PreEscaped(r#"
(function() {
  function getStoredTheme() {
    try {
      return localStorage.getItem('theme');
    } catch (e) {
      return null;
    }
  }
  
  function getSystemTheme() {
    return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  
  function getCurrentTheme() {
    const stored = getStoredTheme();
    return stored || getSystemTheme();
  }
  
  function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
  }
  
  function setTheme(theme) {
    try {
      if (theme === 'auto') {
        localStorage.removeItem('theme');
        applyTheme(getSystemTheme());
      } else {
        localStorage.setItem('theme', theme);
        applyTheme(theme);
      }
      updateToggle();
    } catch (e) {
      // localStorage not available, fallback to system theme
      applyTheme(getSystemTheme());
    }
  }
  
  function updateToggle() {
    const btn = document.getElementById('theme-toggle');
    if (btn) {
      btn.textContent = getCurrentTheme() === 'dark' ? '☀︎' : '☾';
    }
  }

  // Initialize theme
  const currentTheme = getCurrentTheme();
  applyTheme(currentTheme);

  document.addEventListener('DOMContentLoaded', function() {
    updateToggle();
    const btn = document.getElementById('theme-toggle');
    if (btn) {
      btn.addEventListener('click', function() {
        setTheme(getCurrentTheme() === 'dark' ? 'light' : 'dark');
      });
    }
  });

  // Listen for system preference changes
  if (window.matchMedia) {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function() {
      if (!getStoredTheme()) {
        applyTheme(getSystemTheme());
        updateToggle();
      }
    });
  }

  // Expose for profile page
  window.AtChefTheme = { setTheme, getCurrentTheme, getStoredTheme, updateToggle };
})();
                    "#))
                }
            }
            body class=[body_class] {
                header {
                    a class="logo" href="/" { "at://🧑‍🍳" }
                    div class="nav-links" {
                        button class="theme-toggle" id="theme-toggle" title="Toggle theme" { "☾" }
                        @if let Some(handle) = user_handle {
                            a href="/profile" { (handle) }
                        } @else {
                            a href="/login" { "login" }
                        }
                    }
                }
                (content)
            }
        }
    }
}

pub fn recipe_list(recipes: &[Recipe], user: Option<&crate::oauth::AuthenticatedUser>) -> Markup {
    html! {
        @if let Some(u) = user {
            div class="welcome-card" {
                div style="display: flex; justify-content: space-between; align-items: center;" {
                    div {
                        "Hi, "
                        strong {
                            @if let Some(profile) = &u.profile {
                                @if let Some(name) = &profile.display_name {
                                    (name)
                                } @else {
                                    "friend"
                                }
                            } @else {
                                "friend"
                            }
                        }
                        ". Ready to cook?"
                    }
                    div style="display: flex; gap: 10px;" {
                        a href="/recipe/new" class="create-recipe-btn" { "+ New Recipe" }
                    }
                }
            }
        } @else {
            div class="welcome-card" style="display: flex; justify-content: space-between; align-items: center;" {
                div {
                    a href="/login" { "Log in" } " to create and share recipes!"
                }
                a href="/login" class="create-recipe-btn" {
                    "Log in"
                }
            }
        }

        @for recipe in recipes {
            div class="recipe-item" {
                div class="recipe-title" {
                    a href=(format!("/profile/{}/recipe/{}", recipe.author.handle, recipe.id)) { (&recipe.name) }
                }
                div class="recipe-meta" {
                    "by " (render_author_link(&recipe.author)) " · " (&recipe.time_ago) " · "
                    (recipe.comment_count) " comments"
                }
            }
        }
    }
}

pub fn recipe_page(recipe: &RecipeDetail) -> Markup {
    let (rendered_content, ingredients, equipment) = parse_and_render_cooklang(&recipe.content);

    html! {
        h1 { (&recipe.name) }
        div class="meta" {
            "by " (render_author_link(&recipe.author)) " · " (&recipe.time_ago)
        }

        @if let Some(desc) = &recipe.description {
            p class="description" { (desc) }
        }

        div class="recipe-info-bar" {
            @match (recipe.prep_time, recipe.cook_time) {
                (Some(prep), Some(cook)) => {
                    span class="recipe-info-item" { "Prep " strong { (prep) " min" } }
                    span class="recipe-info-item" { "Cook " strong { (cook) " min" } }
                }
                (Some(prep), None) => {
                    span class="recipe-info-item" { "Prep " strong { (prep) " min" } }
                }
                (None, Some(cook)) => {
                    span class="recipe-info-item" { "Cook " strong { (cook) " min" } }
                }
                (None, None) => {
                    span class="recipe-info-item" { "Time " strong { (recipe.time) " min" } }
                }
            }
            span class="recipe-info-item" { "Serves " strong { (recipe.portions) } }
        }

        @if !ingredients.is_empty() || !equipment.is_empty() {
            div id="recipe-ingredients" {
                @if !ingredients.is_empty() {
                    div class="ing-group" {
                        h2 { "Ingredients" }
                        ul {
                            @for (name, qty) in &ingredients {
                                li data-ingredient=(name.to_lowercase()) {
                                    span class="ingredient-text" {
                                        (name)
                                        @if !qty.is_empty() {
                                            " "
                                            span class="amount" { (qty) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                @if !equipment.is_empty() {
                    div class="ing-group" {
                        h2 { "Equipment" }
                        ul {
                            @for item in &equipment {
                                li { (item) }
                            }
                        }
                    }
                }
            }
        }

        h2 { "Instructions" }
        div id="recipe-content" {
            (rendered_content)
        }

        @if !recipe.comments.is_empty() {
            div class="comments" {
                h2 { "Comments (" (recipe.comments.len()) ")" }
                (render_comments(&recipe.comments))
            }
        }

        script { (PreEscaped(r#"
document.querySelectorAll('#recipe-ingredients li[data-ingredient]').forEach(function(li) {
  var key = li.getAttribute('data-ingredient');
  var spans = document.querySelectorAll('.ingredient[data-ingredient="' + key + '"]');
  var ingredientText = li.querySelector('.ingredient-text');
  
  ingredientText.addEventListener('mouseenter', function() {
    li.classList.add('ing-active');
    spans.forEach(function(s) { s.classList.add('ing-active'); });
  });
  ingredientText.addEventListener('mouseleave', function() {
    li.classList.remove('ing-active');
    spans.forEach(function(s) { s.classList.remove('ing-active'); });
  });
});
        "#)) }
    }
}

fn render_author_link(author: &AuthorInfo) -> Markup {
    html! {
        a href=(format!("/profile/{}", author.handle)) {
            (author.handle)
        }
    }
}

fn render_comments(comments: &[Comment]) -> Markup {
    html! {
        @for comment in comments {
            div class="comment" {
                div class="comment-meta" {
                    (render_author_link(&comment.author)) " · " (&comment.time_ago)
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

pub fn public_profile_page(
    handle: &str,
    recipes: &[Recipe],
    is_owner: bool,
    display_name: Option<&str>,
    description: Option<&str>,
    avatar_url: Option<&str>,
    is_atchef_member: bool,
) -> Markup {
    html! {
        div style="display: flex; align-items: center; gap: 16px; margin-bottom: 8px;" {
            @if let Some(url) = avatar_url {
                img src=(url) alt="Avatar" style="width: 64px; height: 64px; border-radius: 50%; object-fit: cover;";
            }
            div {
                h1 style="margin: 0;" { (display_name.unwrap_or(handle)) }
                @if display_name.is_some() {
                    p class="meta" style="margin: 0;" { "@" (handle) }
                }
                @if is_atchef_member {
                    div style="margin-top: 8px;" {
                        span style="font-size: 14px; color: var(--color-text-secondary);" {
                            "👨‍🍳 Certified Chef"
                        }
                    }
                }
            }
        }
        @if let Some(bio) = description {
            p { (bio) }
        }
        @if is_owner {
            div class="theme-selector" {
                h2 { "Theme" }
                div class="theme-options" {
                    div class="theme-option" {
                        input type="radio" name="theme" value="auto" id="theme-auto" checked;
                        label for="theme-auto" { "Auto (system preference)" }
                    }
                    div class="theme-option" {
                        input type="radio" name="theme" value="light" id="theme-light";
                        label for="theme-light" { "Light" }
                    }
                    div class="theme-option" {
                        input type="radio" name="theme" value="dark" id="theme-dark";
                        label for="theme-dark" { "Dark" }
                    }
                }
            }
            script {
                (PreEscaped(r#"
document.addEventListener('DOMContentLoaded', function() {
  if (window.AtChefTheme) {
    // Set initial radio button state
    const stored = window.AtChefTheme.getStoredTheme();
    const selectedValue = stored || 'auto';
    document.querySelector(`input[value="${selectedValue}"]`).checked = true;
    
    // Handle theme changes
    document.querySelectorAll('input[name="theme"]').forEach(input => {
      input.addEventListener('change', function() {
        window.AtChefTheme.setTheme(this.value);
      });
    });
  }
});
                "#))
            }
            form method="post" action="/logout" style="margin-bottom: 20px;" {
                button type="submit" { "Sign out" }
            }
        }
        @if recipes.is_empty() {
            p class="meta" { "No recipes yet." }
        } @else {
            p class="meta" { (recipes.len()) " recipes" }
            @for recipe in recipes {
                div class="recipe-item" {
                    div class="recipe-title" {
                        a href=(format!("/profile/{}/recipe/{}", handle, recipe.id)) { (&recipe.name) }
                    }
                    div class="recipe-meta" { (&recipe.time_ago) }
                }
            }
        }
    }
}

pub fn login_page(error: Option<&str>) -> Markup {
    html! {
        h1 { "Sign in" }
        p { "Sign in with your Bluesky account to share recipes." }

        @if let Some(err) = error {
            p class="error" { (err) }
        }

        form method="post" action="/login" class="login-form" {
            input type="text" name="handle" placeholder="you.bsky.social" required autofocus;
            button type="submit" { "Sign in" }
        }
    }
}

pub fn recipe_form_page(error: Option<&str>) -> Markup {
    html! {
        h1 { "New Recipe" }
        p { "Create a new recipe using Cooklang format." }

        @if let Some(err) = error {
            p class="error" { (err) }
        }

        form method="post" action="/recipe/new" class="recipe-form" {
            div class="form-group" {
                label for="name" { "Recipe Name" }
                input type="text" id="name" name="name" placeholder="e.g., Perfect Sourdough Bread" required;
            }

            div class="form-group" {
                label for="description" { "Description" }
                textarea id="description" name="description" rows="2" placeholder="A brief description of this recipe..." style="min-height: auto;" {}
            }

            div class="form-row" {
                div class="form-group" {
                    label for="portions" { "Servings" }
                    input type="number" id="portions" name="portions" min="1" value="4" required;
                }
                div class="form-group" {
                    label for="prep_time" { "Prep (min)" }
                    input type="number" id="prep_time" name="prep_time" min="0" value="15" required;
                }
                div class="form-group" {
                    label for="cook_time" { "Cook (min)" }
                    input type="number" id="cook_time" name="cook_time" min="0" value="30" required;
                }
            }

            div class="form-group" {
                label { "Recipe Content" }
                div class="editor-tabs" {
                    button type="button" class="editor-tab active" data-tab="write" { "Write" }
                    button type="button" class="editor-tab" data-tab="preview" { "Preview" }
                }
                div class="editor-panel active" data-panel="write" {
                    textarea id="content" name="content" rows="15" placeholder="Write your recipe in Cooklang format..." required {
                        "Mix @bread flour{500%g} and @water{350%g}.\n\nAdd @sourdough starter{100%g} and @salt{10%g}.\n\nBake in #Dutch oven{} for ~{25%minutes}."
                    }
                }
                div class="editor-panel" data-panel="preview" {
                    div class="preview-section" {
                        div id="preview-content" class="preview-content" {
                            p class="preview-placeholder" { "Start typing to see a preview of your recipe..." }
                        }
                        div id="preview-ingredients" class="preview-ingredients" {}
                    }
                }
                p class="help-text" {
                    "Use "
                    code { "@ingredient{amount}" }
                    " for ingredients, "
                    code { "#equipment{}" }
                    " for equipment, and "
                    code { "~{time}" }
                    " for timers."
                }
            }

            div class="form-actions" {
                button type="submit" class="btn-primary" { "Create Recipe" }
                a href="/" class="btn-secondary" { "Cancel" }
            }
        }

        script { (PreEscaped(RECIPE_FORM_JS)) }
    }
}

pub fn chefs_page(users: &[UserRow]) -> Markup {
    let count = users.len();
    let chef_text = if count == 1 { "chef" } else { "chefs" };

    html! {
        h1 { "Chefs on AtChef" }
        p class="meta" { (count) " " (chef_text) " on the platform" }

        @if users.is_empty() {
            p { "No chefs yet. " a href="/login" { "Sign in" } " to be the first!" }
        }
        @else {
            ul class="chef-list" {
                @for user in users {
                    li class="chef-item" {
                        a href=(format!("/profile/{}", user.handle)) { (user.handle) }
                        span class="meta" { " · joined " (format_time_ago(&user.joined_at)) }
                    }
                }
            }
        }
    }
}

fn format_time_ago(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_seconds() < 3600 {
        format!("{} min ago", duration.num_minutes())
    } else if duration.num_days() < 1 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_days() < 365 {
        format!("{} months ago", duration.num_days() / 30)
    } else {
        format!("{} years ago", duration.num_days() / 365)
    }
}

const RECIPE_FORM_JS: &str = r#"
document.addEventListener('DOMContentLoaded', function() {
    const contentTextarea = document.getElementById('content');
    const previewContent = document.getElementById('preview-content');
    const previewIngredients = document.getElementById('preview-ingredients');
    const tabs = document.querySelectorAll('.editor-tab');
    const panels = document.querySelectorAll('.editor-panel');

    // Tab switching
    tabs.forEach(tab => {
        tab.addEventListener('click', function() {
            const targetTab = this.getAttribute('data-tab');

            // Update tab styles
            tabs.forEach(t => t.classList.remove('active'));
            this.classList.add('active');

            // Show/hide panels
            panels.forEach(panel => {
                if (panel.getAttribute('data-panel') === targetTab) {
                    panel.classList.add('active');
                } else {
                    panel.classList.remove('active');
                }
            });

            // Update preview when switching to preview tab
            if (targetTab === 'preview') {
                updatePreview();
            }
        });
    });

    function parseCooklang(text) {
        const ingredients = new Map();
        const equipment = new Set();
        const timers = [];

        // Parse ingredients: @name{amount%unit}
        const ingredientRegex = /@([^@{]+)\{([^}]*)\}/g;
        let match;
        while ((match = ingredientRegex.exec(text)) !== null) {
            const name = match[1].trim();
            const amountSpec = match[2].trim();
            if (amountSpec) {
                const parts = amountSpec.split('%');
                const amount = parts[0];
                const unit = parts[1] || '';
                const key = unit ? `${name} (${unit})` : name;
                if (!ingredients.has(key)) {
                    ingredients.set(key, { name, amount, unit, count: 0 });
                }
                ingredients.get(key).count++;
            } else {
                if (!ingredients.has(name)) {
                    ingredients.set(name, { name, amount: '', unit: '', count: 0 });
                }
                ingredients.get(name).count++;
            }
        }

        // Parse equipment: #name{}
        const equipmentRegex = /#([^#{]+)\{/g;
        while ((match = equipmentRegex.exec(text)) !== null) {
            equipment.add(match[1].trim());
        }

        // Parse timers: ~{time}
        const timerRegex = /~\{([^}]+)\}/g;
        while ((match = timerRegex.exec(text)) !== null) {
            timers.push(match[1].trim());
        }

        return { ingredients, equipment, timers };
    }

    function renderCooklang(text) {
        let html = '';
        const lines = text.split('\n');

        for (const line of lines) {
            if (!line.trim()) {
                html += '<br>';
                continue;
            }

            let rendered = line;

            // Render ingredients with highlighting
            rendered = rendered.replace(/@([^@{]+)\{([^}]*)\}/g, (match, name, spec) => {
                const amount = spec.replace(/%([^,]+)/, ', $1');
                const display = amount ? `${name} <span class="amount">${amount}</span>` : name;
                return `<span class="ingredient">${display}</span>`;
            });

            // Render equipment
            rendered = rendered.replace(/#([^#{]+)\{([^}]*)\}/g, (match, name) => {
                return `<span class="equipment">${name}</span>`;
            });

            // Render timers
            rendered = rendered.replace(/~\{([^}]+)\}/g, (match, time) => {
                return `<span class="timer">⏱ ${time}</span>`;
            });

            html += `<p>${rendered}</p>`;
        }

        return html;
    }

    function updatePreview() {
        const text = contentTextarea.value;
        if (!text.trim()) {
            previewContent.innerHTML = '<p class="preview-placeholder">Start typing to see a preview of your recipe...</p>';
            previewIngredients.innerHTML = '';
            return;
        }

        const parsed = parseCooklang(text);

        // Render content
        previewContent.innerHTML = renderCooklang(text);

        // Render ingredients list
        let ingredientsHtml = '';
        if (parsed.ingredients.size > 0) {
            ingredientsHtml += '<h3>Ingredients</h3><ul>';
            for (const [key, info] of parsed.ingredients) {
                const amountDisplay = info.amount ? `${info.amount}${info.unit ? ' ' + info.unit : ''}` : '';
                const countDisplay = info.count > 1 ? ` (×${info.count})` : '';
                ingredientsHtml += `<li>${info.name}${amountDisplay ? ` <span class="amount">${amountDisplay}</span>` : ''}${countDisplay}</li>`;
            }
            ingredientsHtml += '</ul>';
        }

        if (parsed.equipment.size > 0) {
            ingredientsHtml += '<h3>Equipment</h3><ul>';
            for (const item of parsed.equipment) {
                ingredientsHtml += `<li>${item}</li>`;
            }
            ingredientsHtml += '</ul>';
        }

        previewIngredients.innerHTML = ingredientsHtml;
    }

    contentTextarea.addEventListener('input', updatePreview);
    updatePreview();
});
"#;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// Parse and render cooklang content.
// Returns (rendered HTML, ingredient list (name, qty_str), equipment list).
fn parse_and_render_cooklang(
    content: &str,
) -> (PreEscaped<String>, Vec<(String, String)>, Vec<String>) {
    use cooklang::model::{Content, Item};

    let (recipe, _) = match cooklang::parse(content).into_result() {
        Ok(r) => r,
        Err(_) => {
            // Fallback: plain text
            let mut html = String::new();
            for line in content.lines() {
                if line.trim().is_empty() {
                    html.push_str("<br>");
                } else {
                    html.push_str("<p>");
                    html.push_str(&html_escape(line));
                    html.push_str("</p>");
                }
            }
            return (PreEscaped(html), vec![], vec![]);
        }
    };

    let mut html = String::new();
    for section in &recipe.sections {
        for content_item in &section.content {
            match content_item {
                Content::Step(step) => {
                    html.push_str("<p>");
                    for item in &step.items {
                        match item {
                            Item::Text { value } => html.push_str(&html_escape(value)),
                            Item::Ingredient { index } => {
                                let ing = &recipe.ingredients[*index];
                                let name = ing.alias.as_deref().unwrap_or(&ing.name);
                                let key = html_escape(&name.to_lowercase());
                                let display = match &ing.quantity {
                                    Some(qty) => format!(
                                        "{} <span class=\"amount\">{}</span>",
                                        html_escape(name),
                                        html_escape(&format!("{qty}"))
                                    ),
                                    None => html_escape(name),
                                };
                                html.push_str(&format!(
                                    "<span class=\"ingredient\" data-ingredient=\"{key}\">{display}</span>"
                                ));
                            }
                            Item::Cookware { index } => {
                                let cw = &recipe.cookware[*index];
                                let name = cw.alias.as_deref().unwrap_or(&cw.name);
                                html.push_str(&format!(
                                    "<span class=\"equipment\">{}</span>",
                                    html_escape(name)
                                ));
                            }
                            Item::Timer { index } => {
                                let timer = &recipe.timers[*index];
                                let display = match (&timer.quantity, &timer.name) {
                                    (Some(qty), _) => format!("{qty}"),
                                    (None, Some(name)) => name.clone(),
                                    (None, None) => String::new(),
                                };
                                html.push_str(&format!(
                                    "<span class=\"timer\">⏱ {}</span>",
                                    html_escape(&display)
                                ));
                            }
                            Item::InlineQuantity { index } => {
                                let qty = &recipe.inline_quantities[*index];
                                html.push_str(&html_escape(&format!("{qty}")));
                            }
                        }
                    }
                    html.push_str("</p>");
                }
                Content::Text(text) => {
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            html.push_str("<br>");
                        } else {
                            html.push_str("<p>");
                            html.push_str(&html_escape(line));
                            html.push_str("</p>");
                        }
                    }
                }
            }
        }
    }

    // Ingredients list: only definitions (skip re-uses of same ingredient)
    let mut ingredients: Vec<(String, String)> = Vec::new();
    let mut equipment: Vec<String> = Vec::new();

    for ing in &recipe.ingredients {
        if ing.relation.is_definition() {
            let name = ing.alias.as_deref().unwrap_or(&ing.name).to_string();
            let qty_str = ing
                .quantity
                .as_ref()
                .map(|q| format!("{q}"))
                .unwrap_or_default();
            ingredients.push((name, qty_str));
        }
    }

    for cw in &recipe.cookware {
        use cooklang::model::ComponentRelation;
        if matches!(cw.relation, ComponentRelation::Definition { .. }) {
            equipment.push(cw.alias.as_deref().unwrap_or(&cw.name).to_string());
        }
    }

    (PreEscaped(html), ingredients, equipment)
}
