use crate::{
    components::container::{calendar_view_container, courses_container},
    middlewares::CookieUserState,
    scraper::{self, ThinCourse},
};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::{AppError, DatabaseAppState};

#[derive(Deserialize, Debug)]
pub struct Search {
    #[serde(flatten)]
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn add_to_calendar<'a, 'b>(
    Path(term): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
    Form(Search { course }): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    // get queried term
    let term: scraper::Term = term.parse().map_err(|err| {
        debug!(?err);
        StatusCode::BAD_REQUEST
    })?;

    // no-op if course is already in state
    if user_state.selection.iter().any(|c| c.0 == course) {
        return Ok((CookieJar::new(), html!()));
    }

    // query db
    let default_sections = state.default_thin_sections(&term, course.clone())?;

    debug!(?default_sections);

    let jar = CookieJar::new().add({
        let mut user_state = user_state.clone();
        user_state.selection.push((course, default_sections));

        Cookie::from(user_state)
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
    user_state: CookieUserState,
    State(_state): State<Arc<DatabaseAppState>>,
    Json(course_crn): Json<Search>,
) -> Markup {
    debug!("rm_from_calendar");
    html! {}
}
