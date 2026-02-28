use crate::db::UserRow;
use crate::models::{Comment, Recipe, RecipeDetail};
use maud::{html, Markup, PreEscaped};

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

.login-form { max-width: 300px; }
.login-form input { width: 100%; padding: 8px; margin-bottom: 10px; border: 1px solid #ccc; border-radius: 4px; }
.login-form button { padding: 8px 16px; background: #5a7d5a; color: #fff; border: none; border-radius: 4px; cursor: pointer; }
.login-form button:hover { background: #4a6d4a; }
.error { color: #b44; margin-bottom: 10px; }
.user-info { font-size: 14px; color: #666; }

dl { margin: 0; }
dt { font-weight: 600; margin-top: 15px; }
dd { margin: 5px 0 0 0; color: #666; }
.nav-links { display: flex; gap: 15px; }

/* Recipe Form Styles */
.recipe-form { margin-top: 20px; }
.form-group { margin-bottom: 20px; }
.form-group label { display: block; font-weight: 600; margin-bottom: 5px; }
.form-group input,
.form-group textarea { width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px; font-family: inherit; font-size: 15px; }
.form-group textarea { resize: vertical; min-height: 200px; font-family: ui-monospace, monospace; font-size: 14px; line-height: 1.5; }
.form-row { display: flex; gap: 20px; }
.form-row .form-group { flex: 1; }
.form-row .form-group input { width: 100%; }
.help-text { font-size: 13px; color: #666; margin-top: 5px; }
.help-text code { background: #f0f0f0; padding: 2px 4px; border-radius: 3px; font-size: 12px; }

.form-actions { display: flex; gap: 15px; align-items: center; margin-top: 25px; }
.btn-primary { padding: 10px 20px; background: #5a7d5a; color: #fff; border: none; border-radius: 4px; cursor: pointer; font-size: 15px; }
.btn-primary:hover { background: #4a6d4a; }
.btn-secondary { padding: 10px 20px; color: #666; text-decoration: none; }
.btn-secondary:hover { color: #222; }

.preview-section { margin-top: 30px; padding: 20px; background: #f9f9f9; border-radius: 8px; border: 1px solid #eee; }
.preview-section h2 { margin-top: 0; }
.preview-content { line-height: 1.7; }
.preview-content p { margin: 0 0 10px 0; }
.preview-placeholder { color: #999; font-style: italic; }
.preview-ingredients { margin-top: 20px; padding-top: 20px; border-top: 1px solid #ddd; }
.preview-ingredients h3 { font-size: 14px; font-weight: 600; margin: 15px 0 8px; color: #444; }
.preview-ingredients ul { margin: 0 0 15px 20px; }
.preview-ingredients li { margin-bottom: 4px; }

.ingredient { color: #2a6; font-weight: 500; }
.ingredient .amount { color: #666; font-weight: normal; }
.equipment { color: #a67; font-weight: 500; }
.timer { color: #67a; background: #eef; padding: 2px 6px; border-radius: 3px; font-size: 14px; }

/* Create Recipe Button */
.create-recipe-btn { padding: 6px 12px; background: #5a7d5a; color: #fff; border-radius: 4px; text-decoration: none; font-size: 14px; }
.create-recipe-btn:hover { background: #4a6d4a; text-decoration: none; }

/* Tab-based recipe editor */
.editor-tabs {
  display: flex;
  border-bottom: 1px solid #ddd;
  margin-bottom: 0;
  gap: 5px;
}
.editor-tab {
  padding: 8px 16px;
  background: #f5f5f5;
  border: 1px solid #ddd;
  border-bottom: none;
  border-radius: 4px 4px 0 0;
  cursor: pointer;
  font-size: 14px;
  color: #666;
}
.editor-tab:hover {
  background: #eee;
}
.editor-tab.active {
  background: #fff;
  color: #222;
  margin-bottom: -1px;
}
.editor-panel {
  display: none;
  border: 1px solid #ddd;
  border-top: none;
  border-radius: 0 0 4px 4px;
  padding: 15px;
}
.editor-panel.active {
  display: block;
}
.editor-panel textarea {
  margin: 0;
  border: none;
  padding: 0;
  min-height: 300px;
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
                style { (PreEscaped(CSS)) }
            }
            body class=[body_class] {
                header {
                    a class="logo" href="/" { "AtChef" }
                    div class="nav-links" {
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
            div class="card" style="background: #f5f5f5; padding: 15px; margin-bottom: 25px; border-radius: 4px;" {
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
            div class="card" style="background: #f5f5f5; padding: 15px; margin-bottom: 25px; border-radius: 4px; display: flex; justify-content: space-between; align-items: center;" {
                div {
                    a href="/login" { "Log in" } " to set your status!"
                }
                a href="/login" class="button" style="padding: 6px 12px; background: #5a7d5a; color: #fff; border: none; border-radius: 4px; text-decoration: none;" {
                    "Log in"
                }
            }
        }

        @for recipe in recipes {
            div class="recipe-item" {
                div class="recipe-title" {
                    a href=(format!("/profile/{}/recipe/{}", recipe.author_handle, recipe.id)) { (&recipe.name) }
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
        h1 { (&recipe.name) }
        div class="meta" {
            "by " (&recipe.author_handle) " · " (&recipe.time_ago)
        }

        div class="info" {
            (recipe.time) " min · " (recipe.portions) " servings"
        }

        div class="content" {
            @for line in recipe.content.lines() {
                @if line.is_empty() {
                    br;
                } @else {
                    p { (line) }
                }
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

pub fn profile_page(user: &crate::oauth::AuthenticatedUser) -> Markup {
    html! {
        h1 { "Your Profile" }

        @if let Some(profile) = &user.profile {
            @if let Some(display_name) = &profile.display_name {
                dl {
                    dt { "Display Name" }
                    dd { (display_name) }
                }
            }

            @if let Some(description) = &profile.description {
                dl {
                    dt { "Bio" }
                    dd { (description) }
                }
            }

            @if let Some(avatar) = &profile.avatar {
                dl {
                    dt { "Avatar" }
                    dd style="font-family: monospace; font-size: 12px; word-break: break-all;" {
                        (format!("{:?}", avatar))
                    }
                }
            }

            @if let Some(banner) = &profile.banner {
                dl {
                    dt { "Banner" }
                    dd style="font-family: monospace; font-size: 12px; word-break: break-all;" {
                        (format!("{:?}", banner))
                    }
                }
            }
        }

        dl {
            dt { "Handle" }
            dd { (&user.handle) }

            dt { "DID" }
            dd style="word-break: break-all;" { (&user.did) }
        }

        form method="post" action="/logout" style="margin-top: 20px;" {
            button type="submit" { "Sign out" }
        }
    }
}

pub fn public_profile_page(handle: &str, recipes: &[Recipe]) -> Markup {
    html! {
        h1 { (handle) }
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

            div class="form-row" {
                div class="form-group" {
                    label for="portions" { "Servings" }
                    input type="number" id="portions" name="portions" min="1" value="4" required;
                }
                div class="form-group" {
                    label for="time" { "Time (minutes)" }
                    input type="number" id="time" name="time" min="1" value="45" required;
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
