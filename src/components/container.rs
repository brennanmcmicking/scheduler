use maud::{html, Markup};

use crate::components;

pub fn render(courses: &Vec<String>) -> Markup {
    html! {
        div id="calendar-container" class="flex flex-col w-full h-full gap-1" {
            div id="calendar-view-container" class="w-full h-1/2 lg:px-1 lg:pt-1" {
                div class="w-full h-full flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
                    (components::calendar::render())
                }
            }
            div id="interactive-container" class="w-full h-1/2 flex flex-row px-1 pb-1 gap-1" {
                div id="search-container" class="flex flex-col gap-1 w-1/2 lg:w-48" {
                    div id="search-text-container" class="w-full h-16 rounded-lg p-1 bg-white dark:bg-neutral-800 text-xl shadow-lg" {
                        input class="form-control w-full h-full lowercase bg-white dark:bg-neutral-800 dark:text-white dark:placeholder:text-neutral-400" type="search"
                            name="search" placeholder="Search..."
                            hx-post="/search"
                            hx-trigger="input changed delay:500ms, search"
                            hx-target="#search-results" {}
                    }
                    div id="search-results" class="w-full h-full rounded-lg p-1 bg-white dark:bg-neutral-800 overflow-y-auto shadow-lg" {
                        (components::search_result::render(courses))
                    }
                }
                div id="courses-container" class="flex flex-col gap-2 grow" {
                    div id="courses-card" class="rounded-lg h-full bg-white dark:bg-neutral-800 flex justify-center items-center p-1 dark:text-white" {
                        "selected courses"
                    }
                }
            }
        }
    }
}
