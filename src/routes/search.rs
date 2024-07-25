use axum::{extract::State, Form};
use maud::{html, Markup};
use serde::Deserialize;

use crate::components;

use super::AppState;

#[derive(Deserialize)]
pub struct Search {
    search: String,
}

pub async fn search<S: AppState>(State(state): State<S>, Form(query): Form<Search>) -> Markup {
    println!("{}", query.search);
    let mut search = String::from(query.search).to_lowercase();
    search.retain(|c| !c.is_whitespace());
    let result: Vec<&String> = state
        .courses()
        .iter()
        .filter(|x| x.contains(&search))
        .collect();
    html! {
        (components::search_result::c(&result))
    }
}
