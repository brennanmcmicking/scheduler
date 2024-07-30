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
    State(_): State<Arc<DatabaseAppState>>,
    Extension(_): CookieUserState,
    Form(query): Form<Search>,
) -> Markup {
    dbg!(&query.search);
    let result: Vec<String> = Vec::new();

    html! {
        (components::search_result::c(&result))
    }
}
