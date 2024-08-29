use crate::{
    components::container::{calendar_view_container, courses_container},
    middlewares::SelectedCourses,
    scraper::{Term, ThinCourse, ThinSection},
};
use axum::{
    extract::{Form, Path, Query, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use maud::html;
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

use super::{AppError, DatabaseAppState, SectionType};

#[derive(Deserialize, Debug)]
pub struct Add {
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn add_to_calendar<'a, 'b>(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected: SelectedCourses,
    Form(Add { course }): Form<Add>,
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
            CookieJar::new().add(new_selected.make_cookie(term)),
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

#[derive(Deserialize, Debug)]
pub struct Remove {
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn rm_from_calendar(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected: SelectedCourses,
    Query(Remove { course }): Query<Remove>,
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

// HANDLER FOR COURSE SECTIONS
#[derive(Debug, PartialEq, Deserialize)]
pub struct SectionQuery {
    course: ThinCourse,

    #[serde(default)] // if no box checked, field does not exist
    crns: Vec<u64>,
}

#[derive(Deserialize, Debug)]
pub struct Update {
    course: ThinCourse,
    crn: String,
}

#[instrument(level = "debug", skip(state))]
pub async fn update_calendar(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    mut selected: SelectedCourses,
    Form(Update { course, crn }): Form<Update>,
) -> Result<impl IntoResponse, AppError> {
    if selected.courses.keys().any(|c| *c == course) {
        let section: ThinSection = crn.into();

        let selection = selected.courses.get_mut(&course).unwrap();

        let t = state.get_section_type(&term, &section)?;
        match t {
            SectionType::Lecture => selection.lecture = section,
            SectionType::Lab => selection.lab = Some(section),
            SectionType::Tutorial => selection.tutorial = Some(section),
        }
    }

    let courses = state.courses(term, &selected.thin_courses())?;

    Ok((
        CookieJar::new().add(selected.make_cookie(term)),
        html! {
            (calendar_view_container(true))
            (courses_container(true, term, &courses, &selected))
        },
    ))
}
