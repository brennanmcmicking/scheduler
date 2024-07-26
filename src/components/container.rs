use maud::{html, Markup};

use crate::components;

pub fn c(courses: &Vec<String>) -> Markup {
    return html! {
        div id="calendar-container" class="flex flex-col w-full h-full" {
            div id="calendar-view-container" class="w-full h-1/2 p-1" {
                div class="w-full h-full flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
                    "calendar"
                }
            }
            div id="interactive-container" class="w-full h-1/2 flex flex-row" {
                div id="search-container" class="flex flex-col gap-2 w-1/2 lg:w-48 p-1" {
                    div id="search-text-container" class="w-full h-16 rounded-lg p-2 bg-white dark:bg-neutral-800 text-xl shadow-lg" {
                        input class="form-control w-full h-full lowercase bg-white dark:bg-neutral-800 dark:text-white dark:placeholder:text-neutral-400" type="search"
                            name="search" placeholder="Search..."
                            hx-post="/search"
                            hx-trigger="input changed delay:500ms, search"
                            hx-target="#search-results" {}
                    }
                    div id="search-results" class="w-full h-full rounded-lg p-1 bg-white dark:bg-neutral-800 overflow-y-auto shadow-lg" {
                        (components::search_result::c(courses))
                    }
                }
                div id="courses-container" class="flex flex-col gap-2 grow p-1" {
                    div id="courses-card" class="rounded-lg h-full bg-white dark:bg-neutral-800 flex justify-center items-center p-1 dark:text-white" {
                        "selected courses"
                    }
                }
            }
        }
    };
}
