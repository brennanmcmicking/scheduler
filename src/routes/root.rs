use axum::extract::State;
use maud::{html, Markup};

use crate::components;

use super::AppState;
// use hypertext::{html_elements, rsx, Renderable, Rendered};

pub async fn root<S: AppState>(State(state): State<S>) -> Markup {
    let courses = state.courses().iter().collect();
    return components::base(html! {
        (components::calendar::c(&courses))
    });
}
