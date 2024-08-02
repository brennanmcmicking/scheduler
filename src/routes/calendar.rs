use crate::{middlewares::CookieUserState, scraper::ThinCourse};
use axum::{
    extract::{Json, State},
    Extension, Form,
};
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::DatabaseAppState;

#[derive(Deserialize, Debug)]
pub struct Search {
    #[allow(dead_code)] // FIXME: after we actually implement add_to_calendar
    pub course: String,
}

// curl
// -H "Content-Type: application/x-www-form-urlencoded"
// -X PUT "http://localhost:8080/calendar"
// -d "crn=123&crn=456"
#[instrument(level = "debug", skip(_state))]
pub async fn add_to_calendar(
    State(_state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
    Form(form): Form<Search>,
) -> Markup {
    debug!("add_to_calendar");
    let mut new_state = user_state.to_owned();
    new_state.selection.push(ThinCourse {
        subject_code: "".to_string(),
        course_code: "".to_string(),
        sections: Vec::new(),
    });

    debug!(?new_state);
    html! {
        p {
            "added course "
        }
    }
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
