use axum::{extract::State, Extension, Form};
use maud::{html, Markup};
use serde::Deserialize;

use crate::{components, middlewares::CookieUserState};

use super::AppState;

#[derive(Deserialize)]
pub struct Search {
    search: String,
}

pub async fn search<S: AppState>(
    State(state): State<S>,
    Extension(user_state): CookieUserState,
    Form(query): Form<Search>,
) -> Markup {
    println!("{}", query.search);
    let mut search = query.search.to_lowercase();
    search.retain(|c| !c.is_whitespace());
    let result: Vec<String> = state
        .courses()
        .iter()
        .filter(|x| {
            x.contains(&search)
                && user_state
                    .selection
                    .clone()
                    .iter()
                    .filter(|course| course.name == **x)
                    .count()
                    == 0
        })
        .map(|course| course.to_owned())
        .collect();

    html! {
        (components::search_result::c(&result))
    }
}
