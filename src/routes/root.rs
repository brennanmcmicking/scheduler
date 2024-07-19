use maud::{html, Markup};

use crate::components;
// use hypertext::{html_elements, rsx, Renderable, Rendered};

pub fn root() -> Markup {
    let mut courses: Vec<String> = Vec::new();
    courses.push("MATH100".to_string());
    courses.push("CSC111".to_string());
    courses.push("ENGR110".to_string());
    courses.push("MATH122".to_string());
    courses.push("MATH110".to_string());
    courses.push("ENGR141".to_string());
    courses.push("CSC225".to_string());
    courses.push("PHYS111".to_string());
    return components::base(html! {
        div class="flex justify-center gap-4 h-4/5" {
            div id="search-container" class="flex flex-col gap-1" {
                div id="search-text-container" class="w-full h-12 rounded-lg border-2 border-black p-1 bg-white" {
                    input class="form-control w-full h-full" type="search"
                        name="search" placeholder="Search..."
                        hx-post="/search"
                        hx-trigger="input changed delay:500ms, search"
                        hx-target="#search-results" {}
                }
                div id="search-results" class="w-full h-full rounded-lg border-2 border-black p-1 bg-white" {
                    (components::search_result::c(courses))
                }
            }
            div id="calendar-view" class="w-3/5 rounded-lg border-2 border-black bg-white" {

            }
        }
    });
}
