use maud::{html, Markup};

use crate::scraper::ThinCourse;

pub fn render(schedule_id: &String, courses: &[ThinCourse]) -> Markup {
    html! {
        div {
            @for course in courses {
                @let course_name = format!("{} {}", course.subject_code, course.course_code);
                form class="flex border-b border-neutral-400 dark:border-neutral-400 justify-between items-center mb-0" {
                    div class="text-xl" {
                        (course_name)
                    }
                    button name="course" value=(course_name)
                    class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 transition rounded-lg h-full p-1 my-1 text-xl"
                    hx-put={"/schedule/" (schedule_id) "/calendar"} hx-swap="multi:#calendar-view,#courses-view" {
                        "add"
                    }
                }
            }
        }
    }
}
