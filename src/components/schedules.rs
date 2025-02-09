use std::cmp::Reverse;

use maud::{html, Markup};

use common::ScheduleWithId;

use crate::common;

pub fn view(mut schedules: Vec<ScheduleWithId>) -> Markup {
    schedules.sort_by_key(|s| Reverse(format!("{}{}", s.schedule.term, s.schedule.name)));
    html!(
        div id="schedules-view" class="flex flex-col gap-2" {
            @for schedule in schedules {
                div class="flex gap-2" {
                    a href={"/schedule/" (schedule.id)} class="grow flex gap-2 justify-between bg-blue-500 dark:bg-blue-600 hover:bg-blue-700 hover:dark:bg-blue-800 rounded-lg transition p-2" {
                        p {(schedule.schedule.name)}
                        p {(schedule.schedule.term.human_display())}
                    }
                    button hx-delete={"/schedule/" (schedule.id)} hx-target="#schedules-view" hx-swap="outerHTML"
                    class="w-10 h-10 flex justify-center items-center bg-red-500 dark:bg-red-600 hover:bg-red-700 hover:dark:bg-red-800 rounded-lg transition p-2" {
                        ("x")
                    }
                }
            }
        }
    )
}
