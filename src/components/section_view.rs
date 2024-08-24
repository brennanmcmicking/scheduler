use jiff::civil::Time;
use maud::{html, Markup};
use tracing::error;

use crate::{
    middlewares::SelectedCourses,
    scraper::{Course, Days, MeetingTime, Section, Term, ThinCourse},
};

pub fn section_card(course: &Course, selected: &SelectedCourses, term: &Term) -> Markup {
    let mut lectures: Vec<&Section> = Vec::new();
    let mut labs: Vec<&Section> = Vec::new();
    let mut tutorials: Vec<&Section> = Vec::new();

    for section in course.sections.iter() {
        let sqn = &section.sequence_code;

        if sqn.starts_with('A') {
            lectures.push(section);
        } else if sqn.starts_with('B') {
            labs.push(section);
        } else if sqn.starts_with('T') {
            tutorials.push(section);
        } else {
            error!(
                "unhandled section type: {}\ndropping section {:?}",
                sqn, section
            );
        }
    }

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

    let url = format!("/term/{}/section", term);

    html! {
        form class="px-3" {
            input type="hidden" name="course" value=(format!("{} {}",&course.subject_code, &course.course_code));

            (section( "lectures",&url, &lectures, &selected_crns))
            (section( "labs",&url, &labs, &selected_crns))
            (section( "tutorials",&url, &tutorials, &selected_crns))

        }
    }
}

fn section(
    section_type: &str,
    put_url: &String,
    sections: &Vec<&Section>,
    selected_crns: &[u64],
) -> Markup {
    html! {
        ul class="flex flex-col gap-3" {

            @for section in sections {
                @let checked = selected_crns.iter().any(|&c| c == section.crn);

                li class="flex flex-col text-white" {

                    hr class="my-3 w-full"{}

                    div class="px-3 flex flex-col gap-3 items-start justify-center" {

                        div class="flex gap-3 justify-start items-center"{

                            label class="cursor-pointer flex gap-2 justify-start items-center" {

                                (checkbox(&put_url, section_type,section.crn, checked))

                                "Section: " (&section.sequence_code)
                            }

                        }

                        h4 class="" {
                            "CRN: "(section.crn)
                        }

                        (meeting_times(&section.meeting_times))

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

fn checkbox(url: &String, name: &str, crn: u64, checked: bool) -> Markup {
    // bc I can't figure out how to inline the `checked` html
    // attribute properly :(
    match checked {
        true => html! {
            input type="radio"
                hx-put=(url)
                name=(name)
                value=(crn)
                checked;
        },
        false => html! {
            input type="radio"
                hx-put=(url)
                name=(name)
                value=(crn);
        },
    }
}

fn meeting_times(meeting_times: &[MeetingTime]) -> Markup {
    html! {
        @if !meeting_times.is_empty() {

            ul class="flex flex-col w-full justify-start items-center" {

                @for meeting_time in meeting_times.iter() {
                    li class="flex justify-between items-center text-sm gap-2 w-full" {
                        span { "Days: "(format_days(&meeting_time.days)) }
                        span { (format_time(&meeting_time.start_time, &meeting_time.end_time)) }
                    }
                }

            }

        } @else {
            span { "Days: N/A" }
            span { "N/A" }
        }

    }
}

fn format_days(days: &Days) -> String {
    let mut day_str = String::with_capacity(6); // at most "MTWThF"

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

    if day_str.is_empty() {
        String::from("N/A")
    } else {
        day_str
    }
}

fn format_time(start_time: &Option<Time>, end_time: &Option<Time>) -> String {
    if start_time.is_some() && end_time.is_some() {
        let pattern = "%I:%M %p"; // formats time to `03:00 PM`

        let start = &start_time.unwrap().strftime(pattern);
        let end = &end_time.unwrap().strftime(pattern);

        format!("{} - {}", start, end)
    } else {
        String::from("N/A")
    }
}
