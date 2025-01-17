use std::sync::Arc;

use axum::{extract::{Query, State}, response::IntoResponse};
use axum_extra::extract::{cookie::Cookie, CookieJar, Form};
use maud::html;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::debug;

use crate::{components, middlewares::{Authority, GoogleCsrfCookie, Session}};

use super::{AppError, DatabaseAppState, Stage};


pub async fn get(State(state): State<Arc<DatabaseAppState>>, session: Option<Session>) -> Result<impl IntoResponse, AppError> {
    // if visiting the login page when already logged in, log them out
    let jar = match session {
        Some(_s) => {
            let cookie = Cookie::build(("session", "")).path("/").removal().build();
            debug!("destroying session cookie");
            CookieJar::new().add(cookie)
        }
        None => CookieJar::new(),
    };
    let discord_link = match state.stage {
        Stage::LOCAL => "https://discord.com/oauth2/authorize?client_id=1324110828810797108&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A8443%2Flogin%2Fdiscord&scope=identify",
        Stage::PROD => "https://discord.com/oauth2/authorize?client_id=1324110828810797108&response_type=code&redirect_uri=https%3A%2F%2Fscheduler.brennanmcmicking.net%2Flogin%2Fdiscord&scope=identify",
    };
    Ok((
        jar,
        components::base(
            html! {
                script src="https://accounts.google.com/gsi/client" async {}
                div id="login-container" class="flex flex-col gap-2 py-2 w-full items-center" {
                    a href=(discord_link) {
                        div class="rounded bg-[#5865F2] flex h-10 p-2 gap-2" {
                            img class="w-10" src="/assets/discord-mark-white.svg" {}
                            p class="whitespace-nowrap" { "Continue with Discord" }
                        }
                    }
                    div id="g_id_onload"
                        data-client_id="839626045148-u695skik1hvq9o41dactp72usr0i9bsh.apps.googleusercontent.com"
                        data-context="signin"
                        data-ux_mode="popup"
                        data-login_uri="/login/google"
                        data-auto_prompt="false" {}
                    div class="g_id_signin w-auto"
                        data-type="standard"
                        data-shape="rectangular"
                        data-theme="outline"
                        data-text="continue_with"
                        data-size="large"
                        data-logo_alignment="left" {}
                }
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
    csrf_cookie: GoogleCsrfCookie,
    Form( GoogleLoginRequest { g_csrf_token, credential }): Form<GoogleLoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    debug!("{}", g_csrf_token);
    debug!("{}", csrf_cookie.value);
    debug!("{}", credential);

    match csrf_cookie.value == g_csrf_token {
        true => {
            let payload = state.google_client.validate_id_token(credential).await.unwrap();
            debug!("{}", payload.sub);
        
            // prepend the identity provider to prevent collisions
            let user_id = format!("google_{}", &payload.sub);
            let session = Session {
                session_id: state.make_session(&user_id).await?,
                user_id,
                username: payload.email.expect("no email provided in google sign in response"),
                authority: Authority::GOOGLE,
            };
            
            let cookie = Cookie::build(("session", session.to_base64()))
                .http_only(true)
                .secure(true)
                .path("/")
                .permanent()
                .build();
            
            Ok((
                CookieJar::new().add(cookie),
                [("location", "/")],
                StatusCode::SEE_OTHER,
            ))
        },
        false => {
            Err(StatusCode::BAD_REQUEST.into())
        }
    }
}

#[derive(Deserialize)]
pub struct DiscordCallback {
    code: String,
}

pub async fn get_discord(
    State(app_state): State<Arc<DatabaseAppState>>,
    Query(DiscordCallback { code }): Query<DiscordCallback>,
) -> Result<impl IntoResponse, AppError> {
    let user = app_state.discord_client.get_user(&code).await?;
    let user_id = format!("discord_{}", user.id);
    let session = Session { 
        session_id: app_state.make_session(&user_id).await?,
        user_id,
        username: user.username,
        authority: Authority::DISCORD,
    };
    let cookie = Cookie::build(("session", session.to_base64()))
        .http_only(true)
        .secure(true)
        .path("/")
        .permanent()
        .build();
    
    Ok((
        CookieJar::new().add(cookie),
        [("location", "/")],
        StatusCode::SEE_OTHER,
    ))
}