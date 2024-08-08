use crate::{
    components::container::courses_container,
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
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
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
        return Ok((CookieJar::new(), courses_container()));
    }

    // query db
    let sections = state.thin_sections(&term, course.clone())?;

    let mut default_sections = HashMap::new();
    for (sequence_code, crn) in sections {
        let (letter, rest) = sequence_code.split_at(1);
        match default_sections.entry(letter.to_string()) {
            Entry::Vacant(e) => {
                e.insert((rest.to_string(), crn));
            }
            Entry::Occupied(mut e) => {
                if *rest < *e.get().0 {
                    e.insert((rest.to_string(), crn));
                }
            }
        }
    }
    let mut default_sections = default_sections.into_iter().collect::<Vec<_>>();
    default_sections.sort_by_cached_key(|t| t.0.clone());
    let default_sections = default_sections
        .into_iter()
        .map(|(_, (_, s))| s)
        .collect::<Vec<_>>();

    debug!(?default_sections);

    let jar = CookieJar::new().add({
        let mut user_state = user_state.clone();
        user_state.selection.push((course, default_sections));

        Cookie::from(user_state)
    });

    Ok((jar, courses_container()))
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
