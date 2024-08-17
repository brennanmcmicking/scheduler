use maud::{html, Markup};

use crate::{
    components,
    scraper::{Course, Section, Term, ThinCourse},
};

pub fn calendar_container(
    term: Term,
    search_courses: &Vec<ThinCourse>,
    courses: &[Course],
) -> Markup {
    let sections = courses.iter().flat_map(|c| &c.sections).collect();
    html! {
        div id="calendar-container" class="flex flex-col w-full h-full lg:flex-row lg:p-1 gap-1" {
            (calendar_view_container(false, &sections))
            div id="interactive-container" class="w-full h-1/2 flex flex-row px-1 pb-1 gap-1 lg:contents" {
                div id="search-container" class="flex flex-col gap-1 h-full grow-0 max-w-48 lg:w-48 lg:shrink-0 lg:order-first" {
                    div id="search-text-container" class="w-full h-16 rounded-lg p-1 bg-white dark:bg-neutral-800 text-xl shadow-lg" {
                        input class="form-control w-full h-full lowercase bg-white dark:bg-neutral-800 dark:text-white dark:placeholder:text-neutral-400" type="search"
                            name="search" placeholder="Search..."
                            hx-post={"/term/" (term) "/search"}
                            hx-trigger="input changed delay:200ms, search"
                            hx-target="#search-results" {}
                    }
                    div id="search-results" class="w-full h-full rounded-lg p-1 bg-white dark:bg-neutral-800 overflow-y-auto shadow-lg" {
                        (components::search_result::render(term, search_courses))
                    }
                }
                (courses_container(false))
            }
        }
    }
}

pub fn calendar_view_container(oob: bool, sections: &Vec<&Section>) -> Markup {
    html! {
        div id="calendar-view-container" hx-swap-oob=[if oob {Some("true")} else {None}] class="w-full h-1/2 lg:h-full" {
            div class="w-full h-full lg:p-1 flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
                (components::calendar::render(sections))
            }
        }
    }
}

pub fn courses_container(oob: bool) -> Markup {
    html! {
        div id="courses-container" hx-swap-oob=[if oob {Some("true")} else {None}] class="flex flex-col gap-2 h-full shrink-0 grow basis-1/2 lg:basis-1/5" {
            div id="courses-card" class="rounded-lg h-full bg-white dark:bg-neutral-800 flex justify-center items-center p-1 dark:text-white" {
                "selected courses"
            }
        }
    }
}
