use maud::{html, Markup};

use crate::{
    middlewares::SelectedCourses,
    scraper::{Course, MeetingTime, ThinCourse},
};

pub fn section_card(course: &Course, selected: &SelectedCourses) -> Markup {
    let thin_course = ThinCourse {
        subject_code: course.subject_code.to_owned(),
        course_code: course.course_code.to_owned(),
    };

    let selected_crns = selected
        .courses
        .get(&thin_course)
        .expect("course not found in selections?")
        .iter()
        .map(|s| s.crn)
        .collect::<Vec<_>>();

    html! {
        ul class="flex flex-col gap-3" {
            @for section in &course.sections {
                @let checked = selected_crns.iter().any(|&c| c == section.crn);

                li class="flex flex-col text-white" {

                    hr class="my-3"{}

                    div class="px-3 flex flex-col gap-3 items-start justify-center" {
                        form class="flex gap-3 justify-start items-center"{
                            label class="cursor-pointer flex gap-2 justify-start items-center" {
                                (check_box(section.sequence_code.as_str(), section.sequence_code.as_str(), checked))
                                "Section: " (section.sequence_code)
                            }
                        }

                        h4 class="" {
                            "CRN: "(section.crn)
                        }

                        (meeting_time(&section.meeting_times))
                        (seats(section.enrollment, section.enrollment_capacity, section.waitlist, section.waitlist_capacity))
                    }
                }
            }
        }
    }
}

fn check_box(name: &str, value: &str, checked: bool) -> Markup {
    match checked {
        true => html! {
            input type="checkbox"
                name=(name)
                value=(value)
                checked;
        },
        false => html! {
            input type="checkbox"
                value=(value)
                name=(name);
        },
    }
}

fn seats(enrolled: u32, enrolled_cap: u32, waitlisted: u32, waitlisted_cap: u32) -> Markup {
    html! {
        div class="flex justify-between items-center w-full" {
            span class="text-sm" {
                "Seats: " (enrolled)"/"(enrolled_cap)
            }

            span class="text-sm" {
                "Waitlist: " (waitlisted)"/"(waitlisted_cap)
            }
        }
    }
}

fn meeting_time(meeting_time: &[MeetingTime]) -> Markup {
    // I'm not sure why this meeting time has to be a vec
    let meeting_time = &meeting_time[0];

    let days = &meeting_time.days;

    let mut day_str = String::new();

    if days.monday {
        day_str.push('M');
    }

    if days.tuesday {
        day_str.push('T');
    }

    if days.wednesday {
        day_str.push('W');
    }

    if days.thursday {
        day_str.push_str("Th");
    }

    if days.friday {
        day_str.push('F');
    }

    let start_time = if let Some(time) = &meeting_time.start_time {
        time.strftime("%I:%M %p").to_string()
    } else {
        "N/A".to_string()
    };

    let end_time = if let Some(time) = &meeting_time.end_time {
        time.strftime("%I:%M %p").to_string()
    } else {
        "N/A".to_string()
    };

    html! {
        div class="flex justify-between items-center text-sm gap-5 w-full"{
            span {
                "Day: "(day_str)
            }

            span {
                (start_time) " - " (end_time)
            }
        }
    }
}
