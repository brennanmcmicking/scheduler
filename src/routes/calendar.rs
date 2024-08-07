use crate::{
    middlewares::CookieUserState,
    scraper::{Term, ThinCourse},
};
use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Form};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use maud::html;
use rusqlite::Connection;
use serde::Deserialize;
use std::{ops::DerefMut, sync::Arc};
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
    let term: Term = form.term.parse().map_err(|err| {
        debug!(?err);
        StatusCode::BAD_REQUEST
    })?;

    // get a db conn
    let conn = state.get_conn(&term).ok_or(StatusCode::NOT_FOUND)?;

    // query db
    let (subject_code, course_code) = query_by_crn(&conn, &form.crn)?;

    // building response
    let found = user_state.selection.iter().any(|thincourse| {
        thincourse.subject_code == subject_code && thincourse.course_code == course_code
    });

    let mut jar = CookieJar::new();
    if !found {
        // new course, new cookie
        let mut user_state = user_state.to_owned();
        user_state.selection.push(ThinCourse {
            subject_code,
            course_code,
            sections: Vec::new(),
        });

        let cookie: Cookie = user_state.into();
        jar = jar.add(cookie);
    }

    Ok((
        jar,
        html! {
            p {
                "added course "
            }
        },
    ))
}

/// curl
/// -H "Content-Type: application/x-www-form-urlencoded"
/// -X DELETE "http://localhost:8080/calendar"
/// -d "crn=23962&term=202501"
#[instrument(level = "debug", skip(state))]
pub async fn rm_from_calendar(
    Extension(mut userstate): CookieUserState,
    State(state): State<Arc<DatabaseAppState>>,
    Form(form): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    let term: Term = form.term.parse().map_err(|err| {
        debug!(?err);
        StatusCode::BAD_REQUEST
    })?;
    let conn = state.get_conn(&term).ok_or(StatusCode::NOT_FOUND)?;
    let (subject, course) = query_by_crn(&conn, &form.crn)?;

    let mut jar = CookieJar::new();
    let filtered_selection = userstate
        .selection
        .iter()
        .filter(|&tc| tc.course_code != course && tc.subject_code != subject)
        .collect::<Vec<_>>();

    let changed = filtered_selection.len() != userstate.selection.len();
    let statuscode = if changed {
        userstate.selection = filtered_selection
            .iter()
            .map(|&s| s.to_owned())
            .collect();
        jar = jar.add(userstate);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::OK
    };

    Ok((statuscode, jar, html! { "UI component here" }))
}

/// return the tuple `(subject_code, course_code)` if exists
fn query_by_crn(
    conn: &impl DerefMut<Target = Connection>,
    crn: &String,
) -> Result<(String, String), AppError> {
    let query_result = conn
        .prepare("SELECT subject_code, course_code FROM section WHERE crn = ?1;")
        .context("failed to prepare courses SQL statement")?
        .query_row([crn], |row| {
            let sub: String = row.get(0)?;
            let course: String = row.get(1)?;
            Ok((sub, course))
        })
        .context("query failed, course not found")
        .map_err(|err| {
            debug!(?err);
            StatusCode::NOT_FOUND
        })?;

    Ok(query_result)
}
