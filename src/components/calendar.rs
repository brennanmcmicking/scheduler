use jiff::{civil::Time, ToSpan};
use maud::{html, Markup};
use tracing::debug;

use crate::scraper::{Day, MeetingTime, Section};

fn render_section_cards(earliest: &Time, latest: &Time, sec: &Section, day: Day) -> Markup {
    let earliest = earliest.hour() as f32 + earliest.minute() as f32 / 60.0;
    let latest = latest.hour() as f32 + latest.minute() as f32 / 60.0 + 0.5;
    let meetings: Vec<&MeetingTime> = sec
        .meeting_times
        .iter()
        .filter(|mt| day.is_in_days(mt.days))
        .collect();
    html!(
        @for m in meetings {
            @match m.start_time.zip(m.end_time) {
                None => {}
                Some((st, et)) => {
                    @let st = st.hour() as f32 + st.minute() as f32 / 60.0;
                    @let et = et.hour() as f32 + et.minute() as f32 / 60.0;
                    @let tp = (st - earliest) / (latest - earliest) * 100.0;
                    @let bp = (latest - et) / (latest - earliest) * 100.0;
                    @let border = if sec.enrollment >= sec.enrollment_capacity {
                        "border-2 border-red-800"
                    } else {
                        "p-2"
                    };
                    div class={"absolute top-[calc(" (tp) "%)] bottom-[calc(" (bp) "%)] h-auto w-full"} {
                        div class={"h-full w-full rounded-lg overflow-y-scroll text-xs lg:text-sm color-red bg-[hsl(" (sec.crn % 360) ",50%,25%)] flex flex-col box-sizing " (border)} {
                            div {
                                (sec.subject_code) " " (sec.course_code)
                            }
                            div {
                                (sec.sequence_code)
                            }
                            div {
                                (sec.enrollment) "/" (sec.enrollment_capacity) " (enrolment)"
                            }
                            div {
                                (sec.waitlist) "/" (sec.waitlist_capacity) " (waitlist)"
                            }
                        }
                    }
                }
            }
        }
    )
}

fn render_day(day: Day, timeslots: &Vec<Time>, sections: &Vec<&Section>) -> Markup {
    let earliest = timeslots.first().unwrap();
    let latest = timeslots.last().unwrap();
    debug!(?day, ?sections);
    html!(
        div class="flex-1 flex flex-col" {
            div class="text-[calc(1.5vh)] lg:text-sm shrink flex justify-center items-center" { (day.to_string().to_lowercase()) }
            div class="relative flex flex-col grow gap-0.5 lg:gap-1" {
                @for _ in timeslots {
                    div class="h-auto grow bg-neutral-100 dark:bg-neutral-600" {  }
                }
                @for section in sections {
                    (render_section_cards(earliest, latest, &section, day))
                }
            }
        }
    )
}

pub fn render(sections: &Vec<&Section>) -> Markup {
    debug!(?sections);

    let meeting_times: Vec<&MeetingTime> = sections.iter().flat_map(|s| &s.meeting_times).collect();
    debug!(?meeting_times);

    let earliest: Time = meeting_times
        .iter()
        .flat_map(|mt| mt.start_time)
        .min()
        .unwrap_or(jiff::civil::time(8, 30, 0, 0));

    let latest = meeting_times
        .iter()
        .flat_map(|mt| mt.end_time)
        .max()
        .unwrap_or(jiff::civil::time(23, 20, 0, 0))
        - 20.minutes();

    debug!(?earliest, ?latest);

    let timeslots = earliest
        .series(30.minutes())
        .take_while(|&t| t <= latest)
        .collect::<Vec<Time>>();

    let saturday = meeting_times
        .iter()
        .filter(|mt| Day::Saturday.is_in_days(mt.days))
        .count()
        > 0;

    html!(
        div id="calendar" class="w-full h-full overflow-y-scroll flex gap-0.5 lg:gap-1" {
            div class="flex flex-col shrink" {
                div class="text-[calc(1.5vh)] lg:text-sm shrink flex justify-center items-center" { "time" }
                div class="relative flex flex-col grow gap-0.5 lg:gap-1" {
                    @for (i, time) in timeslots.iter().enumerate() {
                        @let display = if i % 2 != 0 {
                            "hidden lg:flex"
                        } else {
                            "flex"
                        };
                        div class={"text-[calc(1.5vh)] lg:text-sm h-auto grow bg-neutral-200 dark:bg-neutral-700 px-1 " (display) " justify-center"} { (time) }
                    }
                }
            }
            @for d in &Day::WEEKDAYS {
                (render_day(*d, &timeslots, sections))
            }
            @if saturday {
                (render_day(Day::Saturday, &timeslots, sections))
            }
        }
    )
}
