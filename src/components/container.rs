use maud::{html, Markup};

use crate::{
    components, scraper::{Course, Section, ThinCourse}
};

pub fn main_container(
    schedule_id: &String,
    search_courses: &[ThinCourse],
    courses: &[Course],
    selected: &Vec<Section>,
) -> Markup {
    html! {
        div id="main-container" class="flex flex-col w-full h-full lg:flex-row lg:p-1 gap-1" {
            div id="calendar-container" class="w-full h-1/2 lg:h-full" {
                div class="w-full h-full lg:p-1 flex justify-center items-center bg-white dark:bg-neutral-800 lg:rounded-lg shadow-xl" {
                    (components::calendar::view(selected, &[]))
                }
            }
            div id="interactive-container" class="w-full h-1/2 flex flex-row px-1 pb-1 gap-1 lg:contents" {
                div id="search-container" class="flex flex-col gap-1 h-full grow-0 max-w-48 lg:w-48 lg:shrink-0 lg:order-first" {
                    div id="search-text-container" class="w-full h-16 rounded-lg p-1 bg-white dark:bg-neutral-800 text-xl shadow-lg" {
                        input class="form-control w-full h-full lowercase bg-white dark:bg-neutral-800 placeholder:text-neutral-800 dark:placeholder:text-neutral-400" type="search"
                            name="search" placeholder="Search..."
                            hx-post={"/schedule/" (schedule_id) "/search"}
                            hx-trigger="input changed delay:200ms, search"
                            hx-target="#search-results" {}
                    }
                    div id="search-results" class="w-full h-full rounded-lg p-1 bg-white dark:bg-neutral-800 overflow-y-auto shadow-lg" {
                        (components::search_result::render(schedule_id, search_courses))
                    }
                }
                section class="h-full overflow-y-hidden shrink-0 grow basis-1/2 lg:basis-1/5 bg-white dark:bg-neutral-800 p-2 rounded-lg" {
                    div id="courses-container" class="h-full overflow-y-scroll" {
                        (components::courses::view(schedule_id, courses, selected))
                    }
                }
            }
        }
    }
}