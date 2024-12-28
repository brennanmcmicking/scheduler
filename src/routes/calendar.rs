use crate::{
    components, middlewares::{Schedule, Session}, scraper::{ThinCourse, ThinSection}
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

use super::{selected_sections, AppError, DatabaseAppState, SectionType};

#[instrument(level = "debug", skip(state))]
pub async fn get_calendar(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    schedule: Schedule
) -> Result<impl IntoResponse, AppError> {
    let courses = state.courses(schedule.term, &schedule.selected.thin_courses())?;
    let sections = selected_sections(&courses, &schedule.selected);

    Ok(html! {
        (components::calendar::view(&sections, &[]))
    })
}

#[derive(Deserialize, Debug)]
pub struct Add {
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn add_to_calendar(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    schedule: Schedule,
    session: Option<Session>,
    Form(Add { course }): Form<Add>,
) -> Result<impl IntoResponse, AppError> {
    let mut selected = schedule.selected.clone();
    let course_exists = selected.courses.keys().any(|c| *c == course);

    let (jar, selected) = if course_exists {
        // no-op if course is already in state
        (CookieJar::new(), selected)
    } else {
        let default_sections = state.default_thin_sections(&schedule.term, course.clone())?;

        selected.courses.insert(course, default_sections);

        let new_schedule = Schedule { 
            name: schedule.name, 
            term: schedule.term, 
            selected: selected.clone() 
        };

        match session {
            Some(sess) => { 
                let _ = state.set_user_schedule(&sess.user_id, &schedule_id, &new_schedule).await;
                (
                    CookieJar::new(),
                    selected,
                )
            },
            None => (
                CookieJar::new().add(new_schedule.make_cookie(schedule_id.clone())),
                selected,
            ) 
        }
    };

    let courses = state.courses(schedule.term, &selected.thin_courses())?;
    let sections = selected_sections(&courses, &selected);

    Ok((
        jar,
        html! {
            (components::calendar::view(&sections, &[]))
            (components::courses::view(&schedule_id, &courses, &sections))
        },
    ))
}

#[derive(Deserialize, Debug)]
pub struct Remove {
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn rm_from_calendar(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    Query(Remove { course }): Query<Remove>,
    schedule: Schedule,
    session: Option<Session>,
) -> Result<impl IntoResponse, AppError> {
    let selected = schedule.selected.clone();
    // no-op if course is not in cookie
    if !selected.courses.keys().any(|c| *c == course) {
        let courses = state.courses(schedule.term, &selected.thin_courses())?;
        let sections = selected_sections(&courses, &selected);

        return Ok((
            CookieJar::new(),
            html! {
                (components::calendar::view(&sections, &[]))
                (components::courses::view(&schedule_id, &courses, &sections))
            },
        ));
    }

    let mut new_selected = selected.clone();
    new_selected
        .courses
        .retain(|thin_course, _| thin_course.course_code != course.course_code);

    let courses = state.courses(schedule.term, &new_selected.thin_courses())?;
    let sections = selected_sections(&courses, &selected);

    let new_schedule = Schedule {
        name: schedule.name,
        term: schedule.term,
        selected: new_selected,
    };

    let jar = match session {
        Some(sess) => {
            let _ = state.set_user_schedule(&sess.user_id, &schedule_id, &new_schedule).await;
            CookieJar::new()
        },
        None => CookieJar::new().add(new_schedule.make_cookie(schedule_id.clone())),
    };

    Ok((
        jar,
        html! {
            (components::calendar::view(&sections, &[]))
            (components::courses::view(&schedule_id, &courses, &sections))
        },
    ))
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct SectionQuery {
    course: ThinCourse,

    #[serde(default)] // if no box checked, field does not exist
    crns: Vec<u64>,
}

#[derive(Deserialize, Debug)]
pub struct Update {
    crn: u64,
}

#[instrument(level = "debug", skip(state))]
pub async fn update_calendar(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    schedule: Schedule,
    session: Option<Session>,
    Form(Update { crn }): Form<Update>,
) -> Result<impl IntoResponse, AppError> {
    let mut selected = schedule.selected.clone();
    let thin_section = ThinSection { crn };
    let section = state.get_section(&schedule.term, &thin_section)?;
    let course = ThinCourse {
        subject_code: section.subject_code.clone(),
        course_code: section.course_code.clone(),
    };

    if selected.courses.keys().any(|c| *c == course) {
        let selection = selected.courses.get_mut(&course).unwrap();

        let section_type: SectionType = section.sequence_code.into();
        match section_type {
            SectionType::Lecture => selection.lecture = thin_section,
            SectionType::Lab => selection.lab = Some(thin_section),
            SectionType::Tutorial => selection.tutorial = Some(thin_section),
        }
    }

    let courses = state.courses(schedule.term, &selected.thin_courses())?;
    let sections = selected_sections(&courses, &selected);

    let new_schedule = Schedule {
        name: schedule.name,
        term: schedule.term,
        selected,
    };

    let jar = match session {
        Some(sess) => {
            let _ = state.set_user_schedule(&sess.user_id, &schedule_id, &new_schedule).await;
            CookieJar::new()
        },
        None => CookieJar::new().add(new_schedule.make_cookie(schedule_id.clone())),
    };
        
    Ok((
        jar,
        html!(
            (components::calendar::view(&sections, &[]))
            (components::courses::view(&schedule_id, &courses, &sections))
        )
    ))
}
