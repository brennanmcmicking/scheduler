use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;

use serde::{Deserialize, Serialize};

use crate::scraper::ThinCourse;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserState {
    pub selection: Vec<ThinCourse>,
}

pub type CookieUserState = Extension<UserState>;

pub async fn parse_cookie(
    cookie: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let user_state = if let Some(raw_state) = cookie.get("state") {
        // parse the cookie here
        // TODO: currently (of my expectation) the cookie contains the
        // comma seperated CRN's. Need to query Malcolm's scraped data
        // for whatever attributes needed for course display.
        dbg!(&raw_state);
        let blank_selection: Vec<ThinCourse> = Vec::new();
        UserState {
            selection: blank_selection,
        }
    } else {
        let blank_selection: Vec<ThinCourse> = Vec::new();
        // blank_selection.push(Course {
        //     name: "csc111".to_string(),
        //     crns: Vec::new(),
        // });
        UserState {
            selection: blank_selection,
        }
    };

    dbg!(&serde_json::to_string(&user_state).unwrap());

    req.extensions_mut().insert(user_state);
    Ok(next.run(req).await)
}
