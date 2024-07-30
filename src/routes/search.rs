use axum::{extract::State, Extension, Form};
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;

use crate::{components, middlewares::CookieUserState};

use super::DatabaseAppState;

#[derive(Deserialize)]
pub struct Search {
    search: String,
}

pub async fn search(
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
    Form(query): Form<Search>,
) -> Markup {
    println!("{}", query.search);
    let mut search = query.search.to_lowercase();
    search.retain(|c| !c.is_whitespace());
    // let result: Vec<String> = state
    //     .courses()
    //     .iter()
    //     .filter(|x| {
    //         x.contains(&search)
    //             && user_state
    //                 .selection
    //                 .clone()
    //                 .iter()
    //                 .filter(|course| course.name == **x)
    //                 .count()
    //                 == 0
    //     })
    //     .map(|course| course.to_owned())
    //     .collect();
    let result: Vec<String> = Vec::new();

    html! {
        (components::search_result::c(&result))
    }
}
