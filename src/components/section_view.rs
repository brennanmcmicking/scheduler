use maud::{html, Markup};

use crate::{
    middlewares::SelectedCourses,
    scraper::{Course, MeetingTime, Term, ThinCourse},
};

pub fn section_card(course: &Course, selected: &SelectedCourses, term: &Term) -> Markup {
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
        form {
            input type="hidden" name="course_code" value=(&course.course_code);

            input type="hidden" name="subject_code" value=(&course.subject_code);

            ul class="flex flex-col gap-3" {

                @for section in &course.sections {
                    @let checked = selected_crns.iter().any(|&c| c == section.crn);

                    li class="flex flex-col text-white" {

                        hr class="my-3"{}

                        div class="px-3 flex flex-col gap-3 items-start justify-center" {

                            div class="flex gap-3 justify-start items-center"{

                                label class="cursor-pointer flex gap-2 justify-start items-center" {

                                    @let url = format!("/term/{}/section", term.to_string());
                                    (checkbox(url.as_str(), section.crn, checked))

                                    "Section: " (&section.sequence_code)
                                }

                            }

                            h4 class="" {
                                "CRN: "(section.crn)
                            }

                            (meeting_time(&section.meeting_times))

                            div class="flex justify-between items-center w-full" {

                                span class="text-sm" {
                                    "Seats: " (section.enrollment)"/"(section.enrollment_capacity)
                                }

                                span class="text-sm" {
                                    "Waitlist: " (section.waitlist)"/"(section.waitlist_capacity)
                                }

                            }
                        }
                    }
                }
            }
        }
    }
}

fn checkbox(url: &str, crn: u64, checked: bool) -> Markup {
    // bc I can't figure out how to inline the `checked` html
    // attribute properly :(
    match checked {
        true => html! {
            input type="checkbox"
                hx-put=(url)
                name="crns"
                value=(crn)
                checked;
        },
        false => html! {
            input type="checkbox"
                hx-put=(url)
                name="crns"
                value=(crn);
        },
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
