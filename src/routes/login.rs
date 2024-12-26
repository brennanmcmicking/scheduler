use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use maud::html;
use reqwest::StatusCode;
use tracing::debug;

use crate::{components, middlewares::Session};

use super::{AppError, DatabaseAppState};

pub async fn get(session: Option<Session>) -> Result<impl IntoResponse, AppError> {
    // if visiting the login page when already logged in, log them out
    let jar = match session {
        Some(_s) => {
            let cookie = Cookie::build(("session", "")).path("/").removal().build();
            debug!("destroying session cookie");
            CookieJar::new().add(cookie)
        }
        None => CookieJar::new(),
    };
    Ok((
        jar,
        components::base(
            html! {
                div id="login-container" {
                    "Hello, World!"
                }
                div {
                    button hx-post="/login/unsafe" {
                        "test login session"
                    }
                }
            },
            None,
        ),
    ))
}

pub async fn post_google() -> Result<impl IntoResponse, AppError> {
    Ok((
        [("location", "https://scheduler.brennanmcmicking.net")],
        StatusCode::MOVED_PERMANENTLY,
    ))
}

pub async fn post_unsafe(
    State(state): State<Arc<DatabaseAppState>>
) -> Result<impl IntoResponse, AppError> {
    let session = state.make_session("asdf").await;
    let cookie = Cookie::build(("session", serde_json::to_string(&session).expect("failed to serialize json")))
        .http_only(true)
        .secure(true)
        .path("/")
        .permanent()
        .build();
    Ok((
        CookieJar::new().add(cookie),
        [("hx-redirect", "/")],
        StatusCode::MOVED_PERMANENTLY,
    ))
}
