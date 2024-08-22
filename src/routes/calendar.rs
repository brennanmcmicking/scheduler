use crate::{
    components::container::{calendar_view_container, courses_container},
    middlewares::SelectedCourses,
    scraper::{Course, Term, ThinCourse},
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
    let course_exists = selected.courses.keys().any(|c| *c == course);

    let (jar, selected) = if course_exists {
        // no-op if course is already in state
        (CookieJar::new(), selected)
    } else {
        let default_sections = state.default_thin_sections(&term, course.clone())?;

        let mut new_selected = selected.clone();
        new_selected.courses.insert(course, default_sections);

        (
            CookieJar::new().add(selected.make_cookie(term)),
            new_selected,
        )
    };

    let courses = state.courses(term, &selected.thin_courses())?;

    Ok((
        jar,
        html! {
            (calendar_view_container(true))
            (courses_container(true, term, &courses, &selected))
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
        let courses = state.courses(term, &selected.thin_courses())?;

        return Ok((
            CookieJar::new(),
            html! {
                (calendar_view_container(true))
                (courses_container(true, term, &courses, &selected))
            },
        ));
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
