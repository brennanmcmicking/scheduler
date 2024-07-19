use axum::Form;
use maud::{html, Markup};
use serde::Deserialize;

use crate::components;

#[derive(Deserialize)]
pub struct Search {
    search: String,
}

pub async fn search(Form(query): Form<Search>) -> Markup {
    let mut courses: Vec<String> = Vec::new();
    courses.push("MATH100".to_string());
    courses.push("CSC111".to_string());
    courses.push("ENGR110".to_string());
    courses.push("MATH122".to_string());
    courses.push("MATH110".to_string());
    courses.push("ENGR141".to_string());
    courses.push("CSC225".to_string());
    courses.push("PHYS111".to_string());
    println!("{}", query.search);
    let search = String::from(query.search);
    let result = courses
        .into_iter()
        .filter(|x| x.contains(&search))
        .collect();
    html! {
        (components::search_result::c(result))
    }
}
