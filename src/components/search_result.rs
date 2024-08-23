use maud::{html, Markup};

use crate::scraper::{Term, ThinCourse};

pub fn render(term: Term, courses: &Vec<ThinCourse>) -> Markup {
    html! {
        div {
            @for course in courses {
                @let course_name = format!("{} {}", course.subject_code, course.course_code);
                form class="flex justify-between items-center my-3" {
                    div class="text-xl dark:text-white" {
                        (course_name)
                    }
                    input type="hidden" name="course_code" value=(course.course_code){}
                    input type="hidden" name="subject_code" value=(course.subject_code){}
                    button name="course" value=(course_name)
                    class="btn btn-sm btn-primary"
                    hx-put={"/term/" (term) "/calendar"} hx-swap="none" {
                        "add"
                    }
                }
                hr class="w-full my-3 last:hidden";
            }
        }
    }
}
