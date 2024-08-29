use crate::{middlewares::SelectedCourses, scraper::Term};
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
    selected: SelectedCourses,
) -> Result<Markup, AppError> {
    debug!("term endpoint called");
    let search_courses = state.thin_courses(term)?;
    let courses = state.courses(term, &selected.thin_courses())?;

    Ok(components::base(html! {
        (components::container::calendar_container(term, &search_courses, &courses, &selected))
    }))
}
