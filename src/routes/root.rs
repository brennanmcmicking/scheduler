use axum::extract::State;
use maud::{html, Markup};

use crate::components;

use super::AppState;
// use hypertext::{html_elements, rsx, Renderable, Rendered};

pub async fn root<S: AppState>(State(state): State<S>) -> Markup {
    let courses = state.courses().iter().collect();
    return components::base(html! {
        div class="flex justify-center gap-4 h-4/5" {
            div id="search-container" class="flex flex-col gap-1" {
                div id="search-text-container" class="w-full h-12 rounded-lg border-2 border-black p-1 bg-white" {
                    input class="form-control w-full h-full lowercase" type="search"
                        name="search" placeholder="Search..."
                        hx-post="/search"
                        hx-trigger="input changed delay:500ms, search"
                        hx-target="#search-results" {}
                }
                div id="search-results" class="w-full h-full rounded-lg border-2 border-black p-1 bg-white" {
                    (components::search_result::c(&courses))
                }
            }
            div id="calendar-view" class="w-3/5 rounded-lg border-2 border-black bg-white" {

            }
        }
    });
}
