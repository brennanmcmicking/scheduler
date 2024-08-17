use crate::{
    middlewares::SelectedCourses,
    scraper::{Term, ThinCourse, ThinSection},
};
use axum::{
    extract::{Path, State},
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

    let mut user_state = selected_courses.clone();
    user_state.courses.insert(course, default_sections);
    let jar = CookieJar::new().add(user_state.make_cookie(term));

    Ok((jar, render_selected_courses(&user_state)))
}

#[instrument(level = "debug", skip(_state))]
pub async fn rm_from_calendar(
    Path(term): Path<Term>,
    State(_state): State<Arc<DatabaseAppState>>,
    selected_courses: SelectedCourses,
    Form(Search { course }): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    // no-op if course is not in cookie
    if !selected_courses.courses.keys().any(|c| *c == course) {
        return Ok((CookieJar::new(), html!()));
    }

    let jar = CookieJar::new().add({
        let mut user_state = selected_courses;
        user_state
            .courses
            .retain(|thin_course, _| thin_course.course_code != course.course_code);

        user_state.make_cookie(term)
    });

    Ok((
        jar,
        html! {
            "Hello world"
        },
    ))
}

/// some components
fn render_selected_courses(selection: &SelectedCourses) -> Markup {
    let courses = &selection.courses;
    html! {
        ul {
            @for (course, selected_sections) in courses {
                @let title = format!("{} {}", course.subject_code, course.course_code);
                div class="flex items-center" {
                    input type="checkbox" {}
                    span class="text-lg" {(title)}
                    button { "Remove" }
                }
                li { (section_card(&selected_sections)) }
            }
        }
    }
}

fn section_card(sections: &Vec<ThinSection>) -> Markup {
    html! {
        ul {
            @for s in sections {
                li {
                    (s.crn)
                }
            }
        }
    }
}
