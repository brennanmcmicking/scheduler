use axum::extract::State;
use axum_extra::extract::CookieJar;
use maud::{html, Markup};

use crate::components;

use super::AppState;

pub async fn root<S: AppState>(State(state): State<S>, cookie: CookieJar) -> Markup {
    let search_results = if let Some(user_state) = cookie.get("state") {
        // parse the cookie here
        // TODO: currently (of my expectation) the cookie contains the
        // comma seperated CRN's. Need to query Malcolm's scraped data
        // for whatever attributes needed for course display.
        dbg!(&user_state);
        let courses: Vec<String> = Vec::new();
        components::search_result::c(&courses)
    } else {
        components::search_result::c(state.courses())
    };

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
                    (search_results)
                }
            }
            div id="calendar-view" class="w-3/5 rounded-lg border-2 border-black bg-white" {
            }
        }
    });
}
