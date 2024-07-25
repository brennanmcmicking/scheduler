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
    let search = String::from(query.search).to_lowercase();
    let result: Vec<String> = state
        .courses()
        .iter()
        .filter(|x| x.contains(&search))
        .map(|course| course.to_owned())
        .collect();

    html! {
        (components::search_result::c(&result))
    }
}
