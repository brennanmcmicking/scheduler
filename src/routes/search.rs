use axum::{extract::State, Extension, Form};
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::{components, middlewares::CookieUserState};

use super::DatabaseAppState;

#[derive(Deserialize, Debug)]
pub struct Search {
    #[allow(dead_code)]
    search: String, // FIXME: after we actually implement add_to_calendar
}

#[instrument(level = "debug")]
pub async fn search(
    State(_): State<Arc<DatabaseAppState>>,
    Extension(_): CookieUserState,
    Form(query): Form<Search>,
) -> Markup {
    debug!("search");
    let result: Vec<String> = Vec::new();

    html! {
        (components::search_result::render(&result))
    }
}
