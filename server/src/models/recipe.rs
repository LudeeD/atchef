pub struct Recipe {
    pub id: String,
    pub name: String,
    pub author_handle: String,
    pub time_ago: String,
    pub comment_count: u32,
}

pub struct RecipeDetail {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub content: String,
    pub portions: u32,
    pub time: u32,
    pub author_handle: String,
    pub time_ago: String,
    pub comments: Vec<Comment>,
}

pub struct Comment {
    #[allow(dead_code)]
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
            name: "Perfect Sourdough Bread".to_string(),
            author_handle: "breadmaster".to_string(),
            time_ago: "3 hours ago".to_string(),
            comment_count: 12,
        },
        Recipe {
            id: "2".to_string(),
            name: "One-Pan Lemon Chicken".to_string(),
            author_handle: "quickmeals".to_string(),
            time_ago: "5 hours ago".to_string(),
            comment_count: 8,
        },
        Recipe {
            id: "3".to_string(),
            name: "Classic Beef Bourguignon".to_string(),
            author_handle: "frenchcooking".to_string(),
            time_ago: "7 hours ago".to_string(),
            comment_count: 15,
        },
        Recipe {
            id: "4".to_string(),
            name: "Homemade Pasta from Scratch".to_string(),
            author_handle: "pastamaker".to_string(),
            time_ago: "8 hours ago".to_string(),
            comment_count: 6,
        },
        Recipe {
            id: "5".to_string(),
            name: "Thai Green Curry".to_string(),
            author_handle: "spicylove".to_string(),
            time_ago: "10 hours ago".to_string(),
            comment_count: 9,
        },
        Recipe {
            id: "6".to_string(),
            name: "New York Style Pizza Dough".to_string(),
            author_handle: "pizzapro".to_string(),
            time_ago: "12 hours ago".to_string(),
            comment_count: 22,
        },
        Recipe {
            id: "7".to_string(),
            name: "Creamy Mushroom Risotto".to_string(),
            author_handle: "italianfood".to_string(),
            time_ago: "14 hours ago".to_string(),
            comment_count: 5,
        },
        Recipe {
            id: "8".to_string(),
            name: "Crispy Korean Fried Chicken".to_string(),
            author_handle: "seoulcooking".to_string(),
            time_ago: "16 hours ago".to_string(),
            comment_count: 18,
        },
        Recipe {
            id: "9".to_string(),
            name: "Classic French Onion Soup".to_string(),
            author_handle: "soupmaster".to_string(),
            time_ago: "18 hours ago".to_string(),
            comment_count: 4,
        },
        Recipe {
            id: "10".to_string(),
            name: "Chocolate Lava Cake".to_string(),
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
            name: "Perfect Sourdough Bread".to_string(),
            content: r#"A crusty, tangy sourdough with an open crumb. This recipe uses a mature starter and a long cold ferment for maximum flavor development.

Mix @bread flour{500%g} and @water{350%g}, autolyse for ~{30%minutes}.

Add @sourdough starter{100%g} and @salt{10%g}, mix until combined.

Perform 4 sets of stretch and folds over ~{2%hours}.

Bulk ferment at room temperature for ~{4-6%hours} until doubled.

Shape into a boule and place in a #banneton.

Cold retard in #refrigerator for ~{12-16%hours}.

Preheat #Dutch oven to 500°F (260°C).

Score and bake covered for ~{20%minutes}, then uncovered for ~{25%minutes}."#.to_string(),
            portions: 1,
            time: 75,
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
            name: "One-Pan Lemon Chicken".to_string(),
            content: r#"Juicy chicken thighs with crispy skin, roasted with lemon and herbs. Everything cooks in one pan for easy cleanup.

Preheat #oven to 425°F (220°C).

Season @chicken thighs{6%bone-in, skin-on} with @salt{}, @pepper{}, @dried thyme{1%tsp}, and @dried oregano{1%tsp}.

Heat @olive oil{2%tbsp} in #oven-safe skillet{} over medium-high heat.

Sear chicken skin-side down for ~{5%minutes} until golden.

Flip chicken, add @garlic{4%cloves, minced} and @lemons{2%sliced}.

Roast for ~{25-30%minutes} until internal temp reaches 165°F.

Rest for ~{5%minutes} and serve."#.to_string(),
            portions: 4,
            time: 45,
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
                name: r.name,
                content: r#"A delicious recipe waiting to be explored.

Prepare all @ingredients{}.

Cook according to your preference.

Serve and enjoy!"#.to_string(),
                portions: 4,
                time: 45,
                author_handle: r.author_handle,
                time_ago: r.time_ago,
                comments: vec![],
            })
        }
    }
}
