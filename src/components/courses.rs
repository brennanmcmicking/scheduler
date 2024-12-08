use maud::{html, Markup};
use tracing::debug;

use crate::scraper::{Course, MeetingTime, Section};

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
                div class={ (monday_bg) " border px-2 dark:border-white border-neutral-400"} {
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


fn small_section_card(schedule_id: &String, section: &Section, selected: bool) -> Markup {
    let color = match selected {
        true => "bg-blue-600 dark:bg-blue-800",
        false => "bg-green-500 dark:bg-green-800 hover:bg-green-600 hover:dark:bg-green-900",
    };

    let meeting_times = &section.meeting_times;
    let crn = section.crn;
    let sequence_code = &section.sequence_code;
    let full = section.enrollment >= section.enrollment_capacity || section.waitlist > 0;

    html!(
        form hx-patch={"/schedule/" (schedule_id) "/calendar" } hx-swap="multi:#calendar-view,#courses-view" class="mb-0" {
            div
            // hx-get={"/schedule/" (schedule_id) "/calendar"} hx-target="#calendar-view" hx-trigger="pointerleave delay:100ms"
            {
                button
                // hx-get={"/schedule/" (schedule_id) "/calendar/preview"} hx-target="#calendar-view" hx-trigger="pointerenter delay:100ms"
                class={(color) " transition p-2 rounded-lg w-full flex flex-col"} name="crn" value=(crn) {
                    div class="font-bold" {
                        (sequence_code)
                    }
                    div class="text-xs" {
                        @if full {
                            "FULL, waitlist: " (section.waitlist) "/" (section.waitlist_capacity)
                        } @else {
                            "seats: " (section.enrollment) "/" (section.enrollment_capacity)
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

fn sections(schedule_id: &String, sections: Vec<&Section>, selected: &[u64]) -> Markup {
    html!(
        div class="flex flex-col gap-2 py-2 border-t" {
            @for section in sections {
                (small_section_card(schedule_id, &section, selected.contains(&section.crn)))
            }
        }

    )
}

pub fn view(
    schedule_id: &String,
    courses: &[Course],
    selected: &[Section],
) -> Markup {
    let selected: Vec<u64> = selected.iter().map(|s| s.crn).collect();
    debug!(?selected);

    html! {
        div id="courses-view" class="flex flex-col gap-2 " {
            @if courses.is_empty() {
                "use the search bar to add some courses"
            } @else {
                div class="flex justify-between gap-2" { 
                    a class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 transition rounded-lg h-full p-1 my-1 text-xl"
                    href={"/share/" (schedule_id)} {
                        "share"
                    }
                    a class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 transition rounded-lg h-full p-1 my-1 text-xl"
                    href="/" {
                        "generate"
                    }
                }
                @for course in courses {
                    @let lectures: Vec<&Section> = course.sections.iter().filter(|s| s.sequence_code.starts_with("A")).collect();
                    @let labs : Vec<&Section>= course.sections.iter().filter(|s| s.sequence_code.starts_with("B")).collect();
                    @let tutorials: Vec<&Section> = course.sections.iter().filter(|s| s.sequence_code.starts_with("T")).collect();
                    div id={"courses-card-" (&course.subject_code) " " (&course.course_code)} class="bg-neutral-200 dark:bg-neutral-700 rounded-lg flex-col justify-center items-center p-1" {
                        div class="w-full flex items-center justify-between overflow-hidden text-xl align-middle" {
                            (&course.subject_code) " " (&course.course_code)
                            form class="mb-0" {
                                button name="course" value={(course.subject_code) " " (course.course_code)}
                                class="bg-red-500 dark:bg-red-600 hover:bg-red-700 hover:dark:bg-red-800 h-full text-xl p-1 rounded-lg"
                                hx-delete={"/schedule/" (schedule_id) "/calendar"} hx-swap="multi:#calendar-view,#courses-view" {
                                    "remove"
                                }
                            }
                        }
                        h3 {
                            (&course.title)
                        }
    
                        @if !lectures.is_empty() {
                            (sections(&schedule_id, lectures, &selected))
                        }
    
                        @if !labs.is_empty() {
                            (sections(&schedule_id, labs, &selected))
                        }
    
                        @if !tutorials.is_empty() {
                            (sections(&schedule_id, tutorials, &selected))
                        }
                    }
                }
            }
        }
    }
}