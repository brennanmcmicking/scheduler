use maud::{html, Markup};
use tracing::debug;

use crate::{
    components,
    scraper::{Course, MeetingTime, Section, Term, ThinCourse},
};

pub fn calendar_container(
    term: Term,
    search_courses: &[ThinCourse],
    courses: &[Course],
    selected: &Vec<Section>,
) -> Markup {
    html! {
        div id="calendar-container" class="flex flex-col w-full h-full lg:flex-row lg:p-1 gap-1" {
            (calendar_view_container(false, selected, &vec![]))
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
                (courses_container(false, term, courses, selected))
            }
        }
    }
}

pub fn calendar_view_container(oob: bool, sections: &Vec<Section>, preview_sections: &Vec<Section>) -> Markup {
    html! {
        div id="calendar-view-container" hx-swap-oob=[if oob {Some("true")} else {None}] class="w-full h-1/2 lg:h-full" {
            div class="w-full h-full lg:p-1 flex justify-center items-center bg-white dark:bg-neutral-800 dark:text-white lg:rounded-lg shadow-xl" {
                (components::calendar::render(sections, preview_sections))
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
        section class="h-full overflow-y-hidden shrink-0 grow basis-1/2 lg:basis-1/5 bg-white dark:bg-neutral-800 p-2 rounded-lg" {
            div id="courses-container" hx-swap-oob=[if oob {Some("true")} else {None}]
                class="flex flex-col gap-2 h-full overflow-y-scroll" {
                @if courses.is_empty() {
                    p class="dark:text-white" {
                        "use the search bar to add some courses"
                    }
                }
                @for course in courses {
                    @let lectures: Vec<&Section> = course.sections.iter().filter(|s| s.sequence_code.starts_with("A")).collect();
                    @let labs : Vec<&Section>= course.sections.iter().filter(|s| s.sequence_code.starts_with("B")).collect();
                    @let tutorials: Vec<&Section> = course.sections.iter().filter(|s| s.sequence_code.starts_with("T")).collect();
                    div id={"courses-card-" (&course.subject_code) " " (&course.course_code)} class="bg-neutral-200 dark:bg-neutral-700 rounded-lg flex-col justify-center items-center p-1 dark:text-white" {
                        div class="w-full flex items-center justify-between overflow-hidden text-xl align-middle" {
                            (&course.subject_code) " " (&course.course_code)
                            form class="mb-0" {
                                button name="course" value={(course.subject_code) " " (course.course_code)}
                                class="bg-red-500 dark:bg-red-600 hover:bg-red-700 hover:dark:bg-red-800 text-black dark:text-white h-full text-xl p-1 rounded-lg"
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
        div class="flex flex-col gap-2 py-2 border-t" {
            @for section in sections {
                (small_section_card(term, &section, selected.contains(&section.crn)))
            }
        }

    )
}

fn small_section_card(term: &Term, section: &Section, selected: bool) -> Markup {
    let color = match selected {
        true => "bg-blue-800",
        false => "bg-green-800",
    };

    let meeting_times = &section.meeting_times;
    let crn = section.crn;
    let sequence_code = &section.sequence_code;

    html!(
        form hx-patch={"/term/" (term) "/calendar" } hx-swap="none" class="mb-0" {
            div
            hx-get={"/term/" (term) "/calendar"} hx-target="#calendar-view-container" hx-trigger="mouseleave delay:100ms"
            {
                button
                hx-get={"/term/" (term) "/calendar/preview"} hx-target="#calendar-view-container" hx-trigger="mouseenter delay:100ms"
                class={(color) " p-2 rounded-lg w-full flex flex-col"} name="crn" value=(crn) {
                    div class="font-bold" {
                        (sequence_code)
                    }
                    div class="text-xs" {
                        "seats: " (section.enrollment) "/" (section.enrollment_capacity)
                        @if section.enrollment >= section.enrollment_capacity || section.waitlist > 0 {
                            ", waitlist: " (section.waitlist) "/" (section.waitlist_capacity)
                        }
                    }
                    div class="flex flex-col" {
                        @for mt in meeting_times {
                            (meeting_time_indicator(mt))
                        }
                    }
                }
            }
        }
    )
}

fn meeting_time_indicator(mt: &MeetingTime) -> Markup {
    let start_time_str = match mt.start_time {
        Some(t) => t.strftime("%-I:%M%P to").to_string(),
        None => "async".to_string(),
    };
    let end_time_str = match mt.end_time {
        Some(t) => t.strftime("%-I:%M%P").to_string(),
        None => "".to_string(),
    };

    let monday_bg = if mt.days.monday {
        "bg-blue-400"
    } else {
        "hidden lg:block bg-slate-400"
    };
    let tuesday_bg = if mt.days.tuesday {
        "bg-blue-400"
    } else {
        "hidden lg:block bg-slate-400"
    };
    let wednesday_bg = if mt.days.wednesday {
        "bg-blue-400"
    } else {
        "hidden lg:block bg-slate-400"
    };
    let thursday_bg = if mt.days.thursday {
        "bg-blue-400"
    } else {
        "hidden lg:block bg-slate-400"
    };
    let friday_bg = if mt.days.friday {
        "bg-blue-400"
    } else {
        "hidden lg:block bg-slate-400"
    };

    html!(
        div class="flex gap-2 justify-between text-xs" {
            div class="flex font-mono justify-center items-center" {
                div class={ (monday_bg) " border px-2"} {
                    "M"
                }
                div class={ (tuesday_bg) " border px-2"} {
                    "T"
                }
                div class={ (wednesday_bg) " border px-2"} {
                    "W"
                }
                div class={ (thursday_bg) " border px-2"} {
                    "R"
                }
                div class={ (friday_bg) " border px-2"} {
                    "F"
                }
            }
            div {
                p {
                    (start_time_str)
                }
                p {
                    (end_time_str)
                }
            }
        }
    )
}
