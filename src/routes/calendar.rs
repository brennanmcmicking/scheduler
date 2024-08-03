use crate::{
    middlewares::CookieUserState,
    scraper::{self, ThinCourse},
};
use anyhow::Context;
use axum::{
    extract::{Json, State},
    http::{header::SET_COOKIE, HeaderName, StatusCode},
    response::{AppendHeaders, IntoResponse},
    Extension, Form,
};
use axum_extra::extract::cookie::Cookie;
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::{AppError, DatabaseAppState};

#[derive(Deserialize, Debug)]
pub struct Search {
    pub term: String,
    pub crn: String,
}

/// curl
/// -X PUT "http://localhost:8080/calendar"
/// -H "Content-Type: application/x-www-form-urlencoded"\
/// -d "crn=23962&term=202501"
#[instrument(level = "debug", skip(state))]
pub async fn add_to_calendar<'a, 'b>(
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
    Form(form): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    // get queried term
    let term: scraper::Term = form.term.parse().map_err(|err| {
        dbg!(err);
        StatusCode::BAD_REQUEST
    })?;

    // get a db conn
    let conn = state.get_conn(&term).ok_or_else(|| {
        // data for term not found
        StatusCode::NOT_FOUND
    })?;

    // query db
    let (subject_code, course_code) = conn
        .prepare("SELECT subject_code, course_code FROM section WHERE crn = ?1;")
        .context("failed to prepare courses SQL statement")?
        .query_row([form.crn], |row| {
            let sub: String = row.get(0)?;
            let course: String = row.get(1)?;
            Ok((sub, course))
        })
        .context("query failed")
        .map_err(|err| {
            dbg!(err);
            StatusCode::NOT_FOUND
        })?;

    // building response
    let header: AppendHeaders<[(HeaderName, String); 1]>;

    if let Some(_) = user_state.selection.iter().find(|&thincourse| {
        thincourse.subject_code == subject_code && thincourse.course_code == course_code
    }) {
        // early return if course is already in the cookie state
        let cookie: Cookie = user_state.try_into()?;
        header = AppendHeaders([(SET_COOKIE, cookie.to_string())]);
    } else {
        // new course, new cookie
        let mut user_state = user_state.to_owned();
        user_state.selection.push(ThinCourse {
            subject_code,
            course_code,
            sections: Vec::new(),
        });

        let cookie: Cookie = user_state.try_into()?;
        header = AppendHeaders([(SET_COOKIE, cookie.to_string())]);
    }

    Ok((
        header,
        html! {
            p {
                "added course "
            }
        },
    ))
}

// curl
// -H "Content-Type: application/json"
// -X DELETE "http://localhost:8080/calendar"
// -d '{"crn": ["123", "456"]}'
#[instrument(level = "debug", skip(_state))]
pub async fn rm_from_calendar(
    user_state: CookieUserState,
    State(_state): State<Arc<DatabaseAppState>>,
    Json(course_crn): Json<Search>,
) -> Markup {
    debug!("rm_from_calendar");
    html! {}
}
