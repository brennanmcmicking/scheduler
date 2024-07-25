use crate::middlewares::{CookieUserState, Course};
use axum::{
    extract::{Json, State},
    Extension, Form,
};
use maud::{html, Markup};
use serde::Deserialize;

use super::AppState;

#[derive(Deserialize, Debug)]
pub struct Search {
    pub course: String,
}

// curl
// -H "Content-Type: application/x-www-form-urlencoded"
// -X PUT "http://localhost:8080/calendar"
// -d "crn=123&crn=456"
pub async fn add_to_calendar<S: AppState>(
    State(_state): State<S>,
    Extension(user_state): CookieUserState,
    Form(form): Form<Search>,
) -> Markup {
    println!("add_to_calendar");
    dbg!(&user_state, &form);
    let mut new_state = user_state.to_owned();
    new_state.selection.push(Course {
        name: form.course,
        crns: Vec::new(),
    });

    dbg!(&new_state);
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
pub async fn rm_from_calendar<S: AppState>(
    user_state: CookieUserState,
    State(_state): State<S>,
    Json(course_crn): Json<Search>,
) -> Markup {
    println!("rm_to_calendar");
    dbg!(&user_state, &course_crn);
    html! {}
}
