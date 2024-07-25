use crate::middlewares::CookieUserState;
use axum::extract::{Extension, State};
use maud::{html, Markup};

use crate::components;

use super::AppState;

pub async fn root<S: AppState>(
    State(state): State<S>,
    Extension(cookie): CookieUserState,
) -> Markup {
    let search_results = if let Some(user_state) = cookie {
        // parse the cookie here
        // TODO: currently (of my expectation) the cookie contains the
        // comma seperated CRN's. Need to query Malcolm's scraped data
        // for whatever attributes needed for course display.
        dbg!(&user_state);
        let courses: Vec<String> = Vec::new();
    } else {
    };

    return components::base(html! {
        (components::calendar::c(state.courses()))
    });
}
