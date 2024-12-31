use std::sync::Arc;

use axum::{debug_handler, extract::State, response::IntoResponse};
use axum_extra::extract::{cookie::Cookie, CookieJar, Form};
use maud::html;
use reqwest::StatusCode;
use serde::Deserialize;
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
                meta name="google-signin-client_id" content="839626045148-u695skik1hvq9o41dactp72usr0i9bsh.apps.googleusercontent.com" {}
                script src="https://accounts.google.com/gsi/client" async {}
                div id="login-container" {
                    "Hello, World!"
                }
                div {
                    button hx-post="/login/unsafe" {
                        "test login session"
                    }
                }
                // div class="g-signin2" data-onsuccess="onGoogleSignIn" {}
                div id="g_id_onload"
                    data-client_id="839626045148-u695skik1hvq9o41dactp72usr0i9bsh.apps.googleusercontent.com"
                    data-context="signin"
                    data-ux_mode="popup"
                    data-login_uri="/login/google"
                    data-auto_prompt="false" {}

                div class="g_id_signin"
                    data-type="standard"
                    data-shape="rectangular"
                    data-theme="outline"
                    data-text="continue_with"
                    data-size="large"
                    data-logo_alignment="left" {}
            },
            None,
        ),
    ))
}

#[derive(Deserialize, Debug)]
pub struct GoogleLoginRequest {
    g_csrf_token: String,
    credential: String,
}

pub async fn post_google(
    State(state): State<Arc<DatabaseAppState>>,
    Form( GoogleLoginRequest { g_csrf_token, credential}): Form<GoogleLoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    debug!("{}", g_csrf_token);
    debug!("{}", credential);

    let payload = state.google_client.validate_id_token(credential).await.unwrap();
    debug!("{}", payload.sub);

    let session = state.make_session(&payload.sub).await;
    let cookie = Cookie::build(("session", serde_json::to_string(&session).expect("failed to serialize json")))
        .http_only(true)
        .secure(true)
        .path("/")
        .permanent()
        .build();
    
    Ok((
        CookieJar::new().add(cookie),
        [("location", "/")],
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
