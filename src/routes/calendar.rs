use crate::{
    components::container::{calendar_view_container, courses_container},
    middlewares::SelectedCourses,
    scraper::{Term, ThinCourse},
};
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
    Form,
};
use axum_extra::extract::CookieJar;
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::{AppError, DatabaseAppState};

#[derive(Deserialize, Debug)]
pub struct Search {
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn add_to_calendar<'a, 'b>(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected_courses: SelectedCourses,
    Form(Search { course }): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    // no-op if course is already in state
    if selected_courses.courses.keys().any(|c| *c == course) {
        return Ok((CookieJar::new(), html!()));
    }

    // query db
    let default_sections = state.default_thin_sections(&term, course.clone())?;

    debug!(?default_sections);

    let jar = CookieJar::new().add({
        let mut user_state = selected_courses.clone();
        user_state.courses.insert(course, default_sections);

        user_state.make_cookie(term)
    });

    Ok((
        jar,
        html! {
            (calendar_view_container(true))
            (courses_container(true))
        },
    ))
}

#[instrument(level = "debug", skip(_state))]
pub async fn rm_from_calendar(
    _selected_courses: SelectedCourses,
    State(_state): State<Arc<DatabaseAppState>>,
    Json(course_crn): Json<Search>,
) -> Markup {
    todo!()
}
