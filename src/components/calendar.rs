use itertools::Itertools;
use jiff::{civil::Time, ToSpan};
use maud::{html, Markup};

use crate::scraper::{Day, MeetingTime, Section};

#[derive(Clone)]
struct RenderableMeetingTime {
    mt: MeetingTime,
    crn: u64,
    subject_code: String,
    course_code: String,
    sequence_code: String,
    preview: bool,
    full: bool,
}

fn has_conflict(meeting: &RenderableMeetingTime, other: &RenderableMeetingTime) -> bool {
    match meeting.mt.start_time.zip(other.mt.start_time).zip(meeting.mt.end_time).zip(other.mt.end_time) {
        Some((((mst, ost), met), oet)) => {
             mst < oet && ost < met
        },
        None => false
    }
}

fn render_section_cards(earliest: &Time, latest: &Time, renderable_meeting: &RenderableMeetingTime, conflicts_before: usize, conflicts_after: usize) -> Markup {
    let earliest = earliest.hour() as f32 + earliest.minute() as f32 / 60.0;
    let latest = latest.hour() as f32 + latest.minute() as f32 / 60.0 + 0.5;
    let num_overlapping: f32 = conflicts_before as f32 + 1.0 + conflicts_after as f32;
    html!(
        @match renderable_meeting.mt.start_time.zip(renderable_meeting.mt.end_time) {
            None => {}
            Some((st, et)) => {
                @let st = st.hour() as f32 + st.minute() as f32 / 60.0;
                @let et = et.hour() as f32 + et.minute() as f32 / 60.0;
                @let tp = (st - earliest) / (latest - earliest) * 100.0;
                @let bp = (latest - et) / (latest - earliest) * 100.0;
                @let lp = (conflicts_before as f32 / num_overlapping) * 100.0;
                @let rp = (conflicts_after as f32 / num_overlapping) * 100.0; 
                @let border = if renderable_meeting.full {
                    " border-2 border-red-800"
                } else {
                    ""
                };
                @let opacity = if renderable_meeting.preview {
                    " opacity-50"
                } else {
                    ""
                };
                div class={"absolute top-[calc(" (tp) "%)] bottom-[calc(" (bp) "%)] left-[calc(" (lp) "%)] right-[calc(" (rp) "%)] h-auto w-auto" (opacity)} {
                    div class={"h-full w-full rounded-lg overflow-y-scroll text-xs lg:text-sm color-red bg-[hsl(" ((renderable_meeting.crn * 10) % 360) ",100%,40%)] dark:bg-[hsl(" ((renderable_meeting.crn * 10) % 360) ",100%,25%)] flex flex-col box-sizing" (border)} {
                        div class="flex justify-between flex-wrap bg-slate-200 dark:bg-slate-800 px-1" {
                            span class="text-xs lg:text-md" {
                                (renderable_meeting.subject_code) " " (renderable_meeting.course_code)
                            }
                            span class="hidden text-xs lg:block" {
                                (renderable_meeting.sequence_code)
                            }
                        }
                    }
                }
            }
        }
    )
}

fn render_day(day: Day, timeslots: &Vec<Time>, sections: &[Section], preview_sections: &[Section]) -> Markup {
    let earliest = timeslots.first().unwrap();
    let latest = timeslots.last().unwrap();

    let renderable_meetings: Vec<RenderableMeetingTime> = [
        sections.iter()
        .flat_map(|s| s.meeting_times.clone().into_iter()
            .map(|mt| RenderableMeetingTime {
                mt,
                crn: s.crn,
                subject_code: s.subject_code.clone(),
                course_code: s.course_code.clone(),
                sequence_code: s.sequence_code.clone(),
                full: s.enrollment == s.enrollment_capacity || s.waitlist > 0,
                preview: false,
            })
        ).collect::<Vec<_>>(),
        preview_sections.iter()
        .flat_map(|s| s.meeting_times.clone().into_iter()
            .map(|mt| RenderableMeetingTime {
                mt,
                crn: s.crn,
                subject_code: s.subject_code.clone(),
                course_code: s.course_code.clone(),
                sequence_code: s.sequence_code.clone(),
                full: s.enrollment == s.enrollment_capacity || s.waitlist > 0,
                preview: true,
            })
        ).collect()
    ].concat()
    .into_iter()
    .filter(|rm| day.is_in_days(rm.mt.days))
    .collect();
    // debug!(?day, ?sections);
    html!(
        div class="flex-1 flex flex-col" {
            div class="text-[calc(1.5vh)] lg:text-sm shrink flex justify-center items-center" { (day.to_string().to_lowercase()) }
            div class="relative flex flex-col grow gap-0.5 lg:gap-1" {
                @for _ in timeslots {
                    div class="h-auto grow bg-neutral-100 dark:bg-neutral-600" {  }
                }
                @for (pos, meeting) in renderable_meetings.iter().enumerate() {
                    @let conflicts_before: usize = renderable_meetings[0..pos].iter().filter(|other| has_conflict(meeting, other)).collect_vec().len();
                    @let conflicts_after: usize = renderable_meetings[pos+1..renderable_meetings.len()].iter().filter(|other| has_conflict(meeting, other)).collect_vec().len();

                    (render_section_cards(earliest, latest, &meeting, conflicts_before, conflicts_after))
                }
            }
        }
    )
}

pub fn view(sections: &[Section], preview_sections: &[Section]) -> Markup {
    // debug!(?sections);

    let meeting_times: Vec<&MeetingTime> = sections.iter().flat_map(|s| &s.meeting_times).chain(preview_sections.iter().flat_map(
        |s| &s.meeting_times
    )).collect();
    // debug!(?meeting_times);

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

    // debug!(?earliest, ?latest);

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
        div id="calendar-view" class="w-full h-full" {
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
                            div class={"text-[calc(1.5vh)] lg:text-sm h-auto grow bg-neutral-200 dark:bg-neutral-700 px-1 " (display) " justify-center"} { (time.strftime("%-I:%M%P")) }
                        }
                    }
                }
                @for d in &Day::WEEKDAYS {
                    (render_day(*d, &timeslots, sections, preview_sections))
                }
                @if saturday {
                    (render_day(Day::Saturday, &timeslots, sections, preview_sections))
                }
            }
        }
    )
}
