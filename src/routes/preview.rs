use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use maud::html;
use serde::Deserialize;

use crate::{
    components::{self}, middlewares::Schedule, scraper::ThinSection
};

use super::{selected_sections, AppError, DatabaseAppState};

fn default_crn() -> u64 {
    0
}

#[derive(Deserialize, Debug)]
pub struct Preview {
    #[serde(default = "default_crn")]
    crn: u64,
}

pub async fn preview(
    Path(_schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    Query(Preview { crn }): Query<Preview>,
    schedule: Schedule
) -> Result<impl IntoResponse, AppError> {
    let selected = schedule.selected;
    let previewed = if crn != 0 && !selected.crns().contains(&crn) {
        let thin_section = ThinSection { crn };
        vec![state.get_section(&schedule.term, &thin_section)?]
    } else {
        vec![]
    };

    let courses = state.courses(schedule.term, &selected.thin_courses())?;
    let sections = selected_sections(&courses, &selected);

    Ok(html!(
        (components::calendar::view(&sections, &previewed))
    ))
}
