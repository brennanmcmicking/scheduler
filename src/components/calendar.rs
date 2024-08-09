use std::fmt::Display;

use maud::{html, Markup};
use tracing::debug;

use crate::scraper::{Day, Days, MeetingTime, Section};

pub struct HSL {
    h: u16,
    s: u8,
    l: u8,
}

impl From<u64> for HSL {
    fn from(value: u64) -> Self {
        HSL {
            h: u16::try_from(value % 360).unwrap(),
            s: 50,
            l: 25,
        }
    }
}

impl Display for HSL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hsl({},{},{})", self.h, self.s, self.l)
    }
}

struct RenderableMeeting {
    top_percent: f32,
    bottom_percent: f32,
    title: String,
    section: String,
    color: HSL,
    enr: u32,
    enr_cap: u32,
    wl: u32,
    wl_cap: u32,
    border: String,
}

fn render_section_cards(earliest: i8, latest: i8, sec: &Section) -> Markup {
    let renderable_meetings: Vec<RenderableMeeting> = sec
        .meeting_times
        .iter()
        .map(|mt| {
            let title: String = format!("{}{}", sec.subject_code, sec.course_code);
            let st = match mt.start_time {
                None => 8.5,
                Some(t) => t.hour() as f32 + t.minute() as f32 / 60.0,
            };
            let et = match mt.end_time {
                None => 23.5,
                Some(t) => t.hour() as f32 + t.minute() as f32 / 60.0,
            };

            let full = sec.enrollment >= sec.enrollment_capacity;
            let border = if full {
                "border-2 border-red-800".to_string()
            } else {
                "p-2".to_string()
            };
            RenderableMeeting {
                top_percent: (st - f32::from(earliest)) / f32::from(latest - earliest) * 100.0,
                bottom_percent: (f32::from(latest) - et) / f32::from(latest - earliest) * 100.0,
                title,
                section: sec.sequence_code.to_string(),
                color: HSL::from(sec.crn),
                enr: sec.enrollment,
                enr_cap: sec.enrollment_capacity,
                wl: sec.waitlist,
                wl_cap: sec.waitlist_capacity,
                border,
            }
        })
        .collect();
    html!(
        @for m in renderable_meetings {
            div class={"absolute top-[calc(" (m.top_percent) "%)] bottom-[calc(" (m.bottom_percent) "%)] h-auto w-full"} {
                div class={"h-full w-full rounded-lg overflow-y-scroll text-xs lg:text-sm color-red bg-[" (m.color) "] flex flex-col box-sizing " (m.border)} {
                    div {
                        (m.title)
                    }
                    div {
                        (m.section)
                    }
                    div {
                        (m.enr) "/" (m.enr_cap) " (enrolment)"
                    }
                    div {
                        (m.wl) "/" (m.wl_cap) " (waitlist)"
                    }
                }
            }
        }
    )
}

fn render_day(
    earliest: i8,
    latest: i8,
    name: String,
    sections: Vec<&Section>,
    timeslots: &Vec<f64>,
) -> Markup {
    debug!(?name, ?sections);
    html!(
        div class="flex-1 flex flex-col" {
            div class="text-[calc(1vh)] lg:text-sm shrink flex justify-center items-center" { (name) }
            div class="relative flex flex-col grow gap-0.5 lg:gap-1" {
                @for _ in timeslots {
                    div class="h-auto grow bg-neutral-600" {  }
                }
                @for section in sections {
                    (render_section_cards(earliest, latest, &section))
                }
            }
        }
    )
}

fn time_to_string(time: f64) -> String {
    let absolute_hour = time.floor();
    let hour = if absolute_hour >= 13.0 {
        absolute_hour - 12.0
    } else {
        absolute_hour
    };
    let minute = if time - absolute_hour == 0.5 {
        "30"
    } else {
        "00"
    };
    let meridiem = if time >= 12.0 { "pm" } else { "am" };
    format!("{hour}:{minute}{meridiem}")
}

pub fn render(sections: &Vec<Section>) -> Markup {
    debug!(?sections);

    let tmp_meeting = Section {
        crn: 12015,
        subject_code: "MATH".to_string(),
        course_code: "100".to_string(),
        sequence_code: "A01".to_string(),
        enrollment: 0,
        enrollment_capacity: 30,
        waitlist: 0,
        waitlist_capacity: 10,
        meeting_times: vec![MeetingTime {
            start_date: "2024-01-01".parse().unwrap(),
            start_time: Some("08:30:00".parse().unwrap()),
            end_date: "2024-04-30".parse().unwrap(),
            end_time: Some("11:20:00".parse().unwrap()),
            days: Days {
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: true,
                friday: false,
                saturday: false,
                sunday: false,
            },
            building: Some("ECS".to_string()),
            room: Some("123".to_string()),
        }],
    };

    let tmp_meeting2 = Section {
        crn: 12072,
        subject_code: "CSC".to_string(),
        course_code: "111".to_string(),
        sequence_code: "A01".to_string(),
        enrollment: 30,
        enrollment_capacity: 30,
        waitlist: 0,
        waitlist_capacity: 10,
        meeting_times: vec![MeetingTime {
            start_date: "2024-01-01".parse().unwrap(),
            start_time: Some("11:30:00".parse().unwrap()),
            end_date: "2024-04-30".parse().unwrap(),
            end_time: Some("12:20:00".parse().unwrap()),
            days: Days {
                monday: false,
                tuesday: true,
                wednesday: true,
                thursday: false,
                friday: true,
                saturday: false,
                sunday: false,
            },
            building: Some("ECS".to_string()),
            room: Some("123".to_string()),
        }],
    };
    let sections = vec![tmp_meeting, tmp_meeting2];

    let earliest = match sections
        .iter()
        .flat_map(|s| &s.meeting_times)
        .min_by_key(|mt| match mt.start_time {
            None => 24,
            Some(x) => x.hour(),
        }) {
        None => 8,
        Some(x) => x.start_time.unwrap_or("08:00:00".parse().unwrap()).hour(),
    };
    let latest = 1 + match sections
        .iter()
        .flat_map(|s| &s.meeting_times)
        .max_by_key(|mt| match mt.end_time {
            None => 7,
            Some(x) => x.hour(),
        }) {
        None => 24,
        Some(x) => x.end_time.unwrap_or("23:00:00".parse().unwrap()).hour(),
    };
    debug!(earliest, latest);
    let timeslots: Vec<f64> =
        Vec::from_iter(((earliest * 2)..(latest * 2)).map(|n| n as f64 * 0.5));

    let mut cols: Vec<Markup> = Vec::new();
    for day in Day::ALL {
        let mut meetings = Vec::new();
        for section in &sections {
            for mt in &section.meeting_times {
                if day.is_in_days(mt.days) {
                    meetings.push(section);
                }
            }
        }

        if Day::WEEKDAYS.contains(&day) || !meetings.is_empty() {
            cols.push(render_day(
                earliest,
                latest,
                day.to_string().to_lowercase(),
                meetings,
                &timeslots,
            ));
        }
    }
    html!(
        div id="calendar" class="w-full h-full overflow-y-scroll flex gap-0.5 lg:gap-1" {
            div class="flex flex-col shrink" {
                div class="text-[calc(1vh)] lg:text-sm shrink flex justify-center items-center" { "time" }
                div class="relative flex flex-col grow gap-0.5 lg:gap-1" {
                    @for time in &timeslots {
                        div class="text-[calc(1vh)] lg:text-sm h-auto grow bg-neutral-700 px-1 flex justify-center items-center" { (time_to_string(*time)) }
                    }
                }
            }
            @for col in &cols {
                (col)
            }
        }
    )
}
