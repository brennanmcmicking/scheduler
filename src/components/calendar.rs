use maud::{html, Markup};

use crate::components;

pub fn c(courses: &Vec<String>) -> Markup {
    return html! {
        div id="calendar-container" class="flex flex-col w-full h-full" {
            div id="calendar-view-container" class="w-full h-1/2 flex justify-center items-center p-1 bg-white" {
                p {
                    "calendar"
                }
            }
            div id="interactive-container" class="w-full h-1/2 flex flex-row" {
                div id="search-container" class="flex flex-col gap-2 w-1/2 lg:w-48 p-1" {
                    div id="search-text-container" class="w-full h-16 rounded-lg border-2 border-black p-2 bg-white text-xl" {
                        input class="form-control w-full h-full lowercase bg-white" type="search"
                            name="search" placeholder="Search..."
                            hx-post="/search"
                            hx-trigger="input changed delay:500ms, search"
                            hx-target="#search-results" {}
                    }
                    div id="search-results" class="w-full h-full rounded-lg border-2 border-black p-1 bg-white overflow-y-auto" {
                        (components::search_result::c(courses))
                    }
                }
                div id="courses-container" class="flex flex-col gap-2 w-1/2 p-1" {
                    "selected courses"
                }
            }
        }
    };
}
