use crate::middlewares::CookieUserState;
use axum::extract::{Json, State};
use maud::{html, Markup};
use serde::Deserialize;

use super::AppState;

#[derive(Deserialize, Debug)]
pub struct Search {
    pub crn: Vec<String>,
}

// curl
// -H "Content-Type: application/json"
// -X PUT "http://localhost:8080/calendar"
// -d '{"crn": ["123", "456"]}'
pub async fn add_to_calendar<S: AppState>(
    user_state: CookieUserState,
    State(_state): State<S>,
    Json(course_crn): Json<Search>,
) -> Markup {
    println!("add_to_calendar");
    dbg!(&user_state, &course_crn);
    html! {}
}

// curl
// -H "Content-Type: application/json"
// -X DELETE "http://localhost:8080/calendar"
// -d '{"crn": ["123", "456"]}'
pub async fn rm_from_calendar<S: AppState>(
    user_state: CookieUserState,
    State(_state): State<S>,
    Json(course_crn): Json<Search>,
) -> Markup {
    println!("rm_to_calendar");
    dbg!(&user_state, &course_crn);
    html! {}
}
