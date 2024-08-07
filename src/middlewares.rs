use core::str;

use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::scraper::ThinCourse;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserState {
    pub selection: Vec<ThinCourse>,
}

pub type CookieUserState = Extension<UserState>;

impl<'a> From<UserState> for Cookie<'a> {
    fn from(userstate: UserState) -> Self {
        let userstate_json =
            serde_json::to_string(&userstate).expect("failed to serialize to json");

        let userstate_base64 = STANDARD_NO_PAD.encode(userstate_json);

        Cookie::build(("state", userstate_base64))
            .http_only(true)
            .secure(true)
            // .max_age(Duration::MAX) // do we want exp date?
            // .domain(value) // TODO: set domain?
            .build()
    }
}

impl Default for UserState {
    fn default() -> Self {
        let selection = Vec::new();
        UserState { selection }
    }
}

impl<'a> TryFrom<Cookie<'a>> for UserState {
    type Error = anyhow::Error;

    fn try_from(cookie: Cookie<'a>) -> Result<Self, Self::Error> {
        let cookie_base64 = cookie.value();
        let cookie_json = STANDARD_NO_PAD.decode(cookie_base64)?;
        let cookie_json = str::from_utf8(cookie_json.as_ref())?;
        let userstate: UserState = serde_json::from_str(cookie_json)?;
        Ok(userstate)
    }
}

pub async fn parse_cookie(
    cookie: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let user_state = match cookie.get("state") {
        Some(raw_state) => UserState::try_from(raw_state.to_owned()).map_err(|err| {
            error!(?err);
            (StatusCode::BAD_REQUEST, String::from("malformed cookie"))
        })?,
        None => Default::default(),
    };

    req.extensions_mut().insert(user_state);

    Ok(next.run(req).await)
}
