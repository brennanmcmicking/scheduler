use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use maud::html;
use serde::Deserialize;
use tracing::debug;

use crate::{
    components::container::calendar_view_container,
    middlewares::SelectedCourses,
    scraper::{Term, ThinCourse, ThinSection},
};

use super::{selected_sections, AppError, DatabaseAppState, SectionType};

fn default_crn() -> u64 {
    0
}

#[derive(Deserialize, Debug)]
pub struct Preview {
    #[serde(default = "default_crn")]
    crn: u64,
}

pub async fn preview(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected: SelectedCourses,
    Query(Preview { crn }): Query<Preview>,
) -> Result<impl IntoResponse, AppError> {
    let previewed = if crn != 0 {
        let thin_section = ThinSection { crn };
        vec![state.get_section(&term, &thin_section)?]
    } else {
        vec![]
    };

    let courses = state.courses(term, &selected.thin_courses())?;
    let sections = selected_sections(&courses, &selected);

    Ok(html!((calendar_view_container(true, &sections, &previewed))))
}
