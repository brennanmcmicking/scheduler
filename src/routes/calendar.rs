use crate::{
    components::container::{calendar_view_container, courses_container},
    middlewares::SelectedCourses,
    scraper::{Term, ThinCourse},
};
use axum::{
    extract::{Form, Path, Query, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use maud::html;
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
    selected: SelectedCourses,
    Form(Search { course }): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    // no-op if course is already in state
    if selected.courses.keys().any(|c| *c == course) {
        return Ok((CookieJar::new(), html!()));
    }

    // query db
    let default_sections = state.default_thin_sections(&term, course.clone())?;

    debug!(?default_sections);

    let mut new_selected = selected.clone();
    new_selected.courses.insert(course, default_sections);

    let jar = CookieJar::new().add(new_selected.make_cookie(term));

    let courses = state.courses(term, &new_selected.thin_courses())?;

    Ok((
        jar,
        html! {
            (calendar_view_container(true))
            (courses_container(true, term, &courses, &new_selected))
        },
    ))
}

#[instrument(level = "debug", skip(state))]
pub async fn rm_from_calendar(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected: SelectedCourses,
    Query(Search { course }): Query<Search>,
) -> Result<impl IntoResponse, AppError> {
    // no-op if course is not in cookie
    if !selected.courses.keys().any(|c| *c == course) {
        return Ok((CookieJar::new(), html!()));
    }

    let mut new_selected = selected.clone();
    new_selected
        .courses
        .retain(|thin_course, _| thin_course.course_code != course.course_code);

    let jar = CookieJar::new().add(new_selected.make_cookie(term));

    let courses = state.courses(term, &new_selected.thin_courses())?;

    Ok((
        jar,
        html! {
            (calendar_view_container(true))
            (courses_container(true, term, &courses, &new_selected))
        },
    ))
}
