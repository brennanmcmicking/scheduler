use axum::{
    extract::{Path, State},
    Form,
};
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::{components, scraper::Term};

use super::{AppError, DatabaseAppState};

#[derive(Deserialize, Debug)]
pub struct Search {
    search: String,
}

#[instrument(level = "debug", skip(state))]
pub async fn search(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    Form(query): Form<Search>,
) -> Result<Markup, AppError> {
    let courses = state.search(term, &query.search)?;
    debug!(?courses);

    Ok(html! {
        (components::search_result::render(term, &courses))
    })
}
