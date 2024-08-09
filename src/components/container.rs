use maud::{html, Markup};
use tracing::debug;

use crate::{
    components,
    scraper::{Course, Section, Term, ThinCourse},
};

pub fn calendar_container(
    term: Term,
    search_courses: &[ThinCourse],
    courses: &[Course],
    selected: &Vec<Section>,
) -> Markup {
    html! {
        div id="calendar-container" class="flex flex-col w-full h-full lg:flex-row lg:p-1 gap-1" {
            // div id="calendar-view-container" class="w-full h-1/2 lg:h-full" {
            //     div class="w-full h-full lg:p-1 flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
            //         (components::calendar::render(&sections))
            //     }
            // }
            div id="interactive-container" class="w-full h-1/2 flex flex-row px-1 pb-1 gap-1 lg:contents" {
                (calendar_view_container(false, selected))
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

pub fn calendar_view_container(oob: bool, sections: &Vec<Section>) -> Markup {
    html! {
        div id="calendar-view-container" hx-swap-oob=[if oob {Some("true")} else {None}] class="w-full h-1/2 lg:h-full" {
            div class="w-full h-full lg:p-1 flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
                (components::calendar::render(sections))
            }
        }
    }
}

pub fn courses_container(
    oob: bool,
    term: Term,
    courses: &[Course],
    selected: &[Section],
) -> Markup {
    let selected: Vec<u64> = selected.iter().map(|s| s.crn).collect();
    debug!(?selected);
    html! {
        section class="h-full overflow-y-hidden shrink-0 grow basis-1/2 lg:basis-1/5 bg-white dark:bg-neutral-800 p-2" {
            div id="courses-container" hx-swap-oob=[if oob {Some("true")} else {None}]
                class="flex flex-col gap-2 h-full overflow-y-scroll" {
                @for course in courses {
                    @let lectures: Vec<&Section> = course.sections.iter().filter(|s| s.sequence_code.starts_with("A")).collect();
                    @let labs : Vec<&Section>= course.sections.iter().filter(|s| s.sequence_code.starts_with("B")).collect();
                    @let tutorials: Vec<&Section> = course.sections.iter().filter(|s| s.sequence_code.starts_with("T")).collect();
                    div id="courses-card" class="bg-neutral-200 dark:bg-neutral-700 rounded-lg flex-col justify-center items-center p-1 dark:text-white" {
                        div class="w-full flex justify-between items-center overflow-hidden rounded-lg shadow" {
                            div class="h-full grow text-xl dark:text-white dark:bg-neutral-600 p-1" {
                                (&course.subject_code) " " (&course.course_code)
                            }
                            form class="mb-0" {
                                button name="course" value={(course.subject_code) " " (course.course_code)}
                                class="bg-red-500 dark:bg-red-600 hover:bg-red-700 hover:dark:bg-red-800 text-black dark:text-white h-full text-xl p-1"
                                hx-delete={"/term/" (term) "/calendar"} hx-swap="none" {
                                    "remove"
                                }
                            }
                        }
                        h3 {
                            (&course.title)
                        }

                        @if !lectures.is_empty() {
                            (sections(&term, lectures, &selected))
                        }

                        @if !labs.is_empty() {
                            (sections(&term, labs, &selected))
                        }

                        @if !tutorials.is_empty() {
                            (sections(&term, tutorials, &selected))
                        }
                    }
                }
            }
        }
    }
}

fn sections(term: &Term, sections: Vec<&Section>, selected: &[u64]) -> Markup {
    html!(
        div class="grid grid-cols-5 gap-2 py-2 border-t" {
            @for section in sections {
                (small_section_card(term, &section.subject_code, &section.course_code, &section.sequence_code, section.crn, selected.contains(&section.crn)))
            }
        }

    )
}

fn small_section_card(
    term: &Term,
    subject_code: &String,
    course_code: &String,
    sequence_code: &String,
    crn: u64,
    selected: bool,
) -> Markup {
    let border_color = match selected {
        true => "border-blue-800",
        false => "border-green-800",
    };
    html!(
        form hx-patch={"/term/" (term) "/calendar" } hx-swap="none" class="mb-0" {
            input name="course" value=(format!("{} {}", subject_code, course_code)) hidden {}
            button class={(border_color) " border-2 bg-green-800 p-2 rounded-lg w-full"} name="crn" value=(crn) {
                (sequence_code)
            }
        }
    )
}
