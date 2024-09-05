use maud::{html, Markup};

use crate::scraper::{Term, ThinCourse};

pub fn render(term: Term, courses: &[ThinCourse]) -> Markup {
    html! {
        div {
            @for course in courses {
                @let course_name = format!("{} {}", course.subject_code, course.course_code);
                form class="flex border-b dark:border-neutral-400 justify-between items-center mb-0" {
                    div class="text-xl dark:text-white" {
                        (course_name)
                    }
                    button name="course" value=(course_name)
                    class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 transition text-black dark:text-white rounded-lg h-full p-1 my-1 text-xl shadow-lg"
                    hx-put={"/term/" (term) "/calendar"} hx-swap="none" {
                        "add"
                    }
                }
            }
        }
    }
}
