pub struct Recipe {
    pub id: String,
    pub title: String,
    pub author_handle: String,
    pub time_ago: String,
    pub comment_count: u32,
}

pub struct RecipeDetail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    pub prep_time: String,
    pub cook_time: String,
    pub servings: u32,
    pub author_handle: String,
    pub time_ago: String,
    pub comments: Vec<Comment>,
}

pub struct Comment {
    pub id: String,
    pub author_handle: String,
    pub text: String,
    pub time_ago: String,
    pub children: Vec<Comment>,
}

pub fn get_mock_recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            id: "1".to_string(),
            title: "Perfect Sourdough Bread".to_string(),
            author_handle: "breadmaster".to_string(),
            time_ago: "3 hours ago".to_string(),
            comment_count: 12,
        },
        Recipe {
            id: "2".to_string(),
            title: "One-Pan Lemon Chicken".to_string(),
            author_handle: "quickmeals".to_string(),
            time_ago: "5 hours ago".to_string(),
            comment_count: 8,
        },
        Recipe {
            id: "3".to_string(),
            title: "Classic Beef Bourguignon".to_string(),
            author_handle: "frenchcooking".to_string(),
            time_ago: "7 hours ago".to_string(),
            comment_count: 15,
        },
        Recipe {
            id: "4".to_string(),
            title: "Homemade Pasta from Scratch".to_string(),
            author_handle: "pastamaker".to_string(),
            time_ago: "8 hours ago".to_string(),
            comment_count: 6,
        },
        Recipe {
            id: "5".to_string(),
            title: "Thai Green Curry".to_string(),
            author_handle: "spicylove".to_string(),
            time_ago: "10 hours ago".to_string(),
            comment_count: 9,
        },
        Recipe {
            id: "6".to_string(),
            title: "New York Style Pizza Dough".to_string(),
            author_handle: "pizzapro".to_string(),
            time_ago: "12 hours ago".to_string(),
            comment_count: 22,
        },
        Recipe {
            id: "7".to_string(),
            title: "Creamy Mushroom Risotto".to_string(),
            author_handle: "italianfood".to_string(),
            time_ago: "14 hours ago".to_string(),
            comment_count: 5,
        },
        Recipe {
            id: "8".to_string(),
            title: "Crispy Korean Fried Chicken".to_string(),
            author_handle: "seoulcooking".to_string(),
            time_ago: "16 hours ago".to_string(),
            comment_count: 18,
        },
        Recipe {
            id: "9".to_string(),
            title: "Classic French Onion Soup".to_string(),
            author_handle: "soupmaster".to_string(),
            time_ago: "18 hours ago".to_string(),
            comment_count: 4,
        },
        Recipe {
            id: "10".to_string(),
            title: "Chocolate Lava Cake".to_string(),
            author_handle: "sweetooth".to_string(),
            time_ago: "20 hours ago".to_string(),
            comment_count: 11,
        },
    ]
}

pub fn get_mock_recipe_detail(id: &str) -> Option<RecipeDetail> {
    match id {
        "1" => Some(RecipeDetail {
            id: "1".to_string(),
            title: "Perfect Sourdough Bread".to_string(),
            description: "A crusty, tangy sourdough with an open crumb. This recipe uses a mature starter and a long cold ferment for maximum flavor development.".to_string(),
            ingredients: vec![
                "500g bread flour".to_string(),
                "350g water".to_string(),
                "100g sourdough starter".to_string(),
                "10g salt".to_string(),
            ],
            steps: vec![
                "Mix flour and water, autolyse for 30 minutes.".to_string(),
                "Add starter and salt, mix until combined.".to_string(),
                "Perform 4 sets of stretch and folds over 2 hours.".to_string(),
                "Bulk ferment at room temperature for 4-6 hours until doubled.".to_string(),
                "Shape into a boule and place in a banneton.".to_string(),
                "Cold retard in refrigerator for 12-16 hours.".to_string(),
                "Preheat Dutch oven to 500°F (260°C).".to_string(),
                "Score and bake covered for 20 minutes, then uncovered for 25 minutes.".to_string(),
            ],
            prep_time: "30 min".to_string(),
            cook_time: "45 min".to_string(),
            servings: 1,
            author_handle: "breadmaster".to_string(),
            time_ago: "3 hours ago".to_string(),
            comments: vec![
                Comment {
                    id: "c1".to_string(),
                    author_handle: "homebaker".to_string(),
                    text: "This is the best sourdough recipe I've tried!".to_string(),
                    time_ago: "2 hours ago".to_string(),
                    children: vec![
                        Comment {
                            id: "c2".to_string(),
                            author_handle: "breadmaster".to_string(),
                            text: "Thanks! The long cold ferment is the secret.".to_string(),
                            time_ago: "1 hour ago".to_string(),
                            children: vec![],
                        },
                    ],
                },
                Comment {
                    id: "c3".to_string(),
                    author_handle: "breadnewbie".to_string(),
                    text: "What if I don't have a Dutch oven?".to_string(),
                    time_ago: "1 hour ago".to_string(),
                    children: vec![
                        Comment {
                            id: "c4".to_string(),
                            author_handle: "breadmaster".to_string(),
                            text: "Use a baking stone with a pan of water underneath for steam.".to_string(),
                            time_ago: "45 minutes ago".to_string(),
                            children: vec![],
                        },
                    ],
                },
            ],
        }),
        "2" => Some(RecipeDetail {
            id: "2".to_string(),
            title: "One-Pan Lemon Chicken".to_string(),
            description: "Juicy chicken thighs with crispy skin, roasted with lemon and herbs. Everything cooks in one pan for easy cleanup.".to_string(),
            ingredients: vec![
                "6 bone-in, skin-on chicken thighs".to_string(),
                "2 lemons, sliced".to_string(),
                "4 cloves garlic, minced".to_string(),
                "2 tbsp olive oil".to_string(),
                "1 tsp dried thyme".to_string(),
                "1 tsp dried oregano".to_string(),
                "Salt and pepper to taste".to_string(),
            ],
            steps: vec![
                "Preheat oven to 425°F (220°C).".to_string(),
                "Season chicken with salt, pepper, thyme, and oregano.".to_string(),
                "Heat oil in oven-safe skillet over medium-high heat.".to_string(),
                "Sear chicken skin-side down for 5 minutes until golden.".to_string(),
                "Flip chicken, add garlic and lemon slices.".to_string(),
                "Roast for 25-30 minutes until internal temp reaches 165°F.".to_string(),
                "Rest for 5 minutes and serve.".to_string(),
            ],
            prep_time: "10 min".to_string(),
            cook_time: "35 min".to_string(),
            servings: 4,
            author_handle: "quickmeals".to_string(),
            time_ago: "5 hours ago".to_string(),
            comments: vec![
                Comment {
                    id: "c5".to_string(),
                    author_handle: "weeknightchef".to_string(),
                    text: "Made this last night - my family loved it!".to_string(),
                    time_ago: "3 hours ago".to_string(),
                    children: vec![],
                },
            ],
        }),
        _ => {
            let recipes = get_mock_recipes();
            recipes.into_iter().find(|r| r.id == id).map(|r| RecipeDetail {
                id: r.id,
                title: r.title,
                description: "A delicious recipe waiting to be explored.".to_string(),
                ingredients: vec![
                    "Ingredient 1".to_string(),
                    "Ingredient 2".to_string(),
                    "Ingredient 3".to_string(),
                ],
                steps: vec![
                    "Prepare all ingredients.".to_string(),
                    "Cook according to your preference.".to_string(),
                    "Serve and enjoy!".to_string(),
                ],
                prep_time: "15 min".to_string(),
                cook_time: "30 min".to_string(),
                servings: 4,
                author_handle: r.author_handle,
                time_ago: r.time_ago,
                comments: vec![],
            })
        }
    }
}
