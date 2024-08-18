use maud::{html, Markup};

use crate::{
    components,
    middlewares::SelectedCourses,
    scraper::{Course, Term, ThinCourse},
};

pub fn calendar_container(
    term: Term,
    search_courses: &[ThinCourse],
    courses: &[Course],
    selected: &SelectedCourses,
) -> Markup {
    html! {
        div id="calendar-container" class="flex flex-col w-full h-full lg:flex-row lg:p-1 gap-1" {
            div id="interactive-container" class="w-full h-1/2 flex flex-row px-1 pb-1 gap-1 lg:contents" {
                (calendar_view_container(false))
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
                (courses_container(false, term, courses, selected))
            }
        }
    }
}

pub fn calendar_view_container(oob: bool) -> Markup {
    html! {
        div id="calendar-view-container" hx-swap-oob=[if oob {Some("true")} else {None}] class="w-full h-1/2 lg:h-full" {
            div class="w-full h-full lg:p-1 flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
                (components::calendar::render())
            }
        }
    }
}

pub fn courses_container(
    oob: bool,
    term: Term,
    courses: &[Course],
    _selected: &SelectedCourses,
) -> Markup {
    // TODO: add selected section list view
    // TODO: add selected section selection endpoints (i.e PUT /term/:term/calendar/section crn=1)
    html! {
        div id="courses-container" hx-swap-oob=[if oob {Some("true")} else {None}] class="flex flex-col gap-2 h-full shrink-0 grow basis-1/2 lg:basis-1/5" {
            @for course in courses {
                div id="courses-card" class="rounded-lg h-full bg-white dark:bg-neutral-800 flex justify-center items-center p-1 dark:text-white" {
                    (&course.subject_code) " " (&course.course_code) " - " (&course.title)
                    form {
                        input type="hidden" name="subject_code" value=(course.subject_code){}
                        input type="hidden" name="course_code" value=(course.course_code){}
                        button name="course" value={(course.subject_code) " " (course.course_code)}
                        class="bg-red-500 dark:bg-red-600 hover:bg-red-700 hover:dark:bg-red-800 text-black dark:text-white rounded-lg h-full p-1 my-1 text-xl shadow-lg"
                        hx-delete={"/term/" (term) "/calendar"} hx-swap="none" {
                            "Remove"
                        }
                    }
                }
            }
        }
    }
}
