use crate::scraper::Term;
use axum::extract::{Path, State};
use maud::{html, Markup};
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::components;

use super::{AppError, DatabaseAppState};

#[instrument(level = "debug", skip(state))]
pub async fn term(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
) -> Result<Markup, AppError> {
    debug!("term");
    let courses = state.courses(term)?;

    Ok(components::base(html! {
        (components::container::calendar_container(term, &courses, &[]))
    }))
}
