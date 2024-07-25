use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;

#[derive(Clone, Debug)]
pub struct UserState {
    pub course_reg_numbers: Vec<String>,
}

pub type CookieUserState = Extension<Option<UserState>>;

pub async fn parse_cookie(
    cookie: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let user_state = if let Some(user_state) = cookie.get("state") {
        // parse the cookie here
        // TODO: currently (of my expectation) the cookie contains the
        // comma seperated CRN's. Need to query Malcolm's scraped data
        // for whatever attributes needed for course display.
        dbg!(&user_state);
        let course_reg_numbers: Vec<String> = Vec::new();
        Some(UserState { course_reg_numbers })
    } else {
        None
    };
    req.extensions_mut().insert(user_state);
    Ok(next.run(req).await)
}
