use maud::{html, Markup};

use crate::components;
// use hypertext::{html_elements, rsx, Renderable, Rendered};

pub fn root() -> Markup {
    let mut courses: Vec<String> = Vec::new();
    courses.push("MATH100".to_string());
    courses.push("CSC111".to_string());
    return components::base(html! {
        // p { "ptag" }
        // (components::button::c(html! { p {"button"}}))
        div class="flex justify-center gap-4 h-4/5" {
            (components::searchResult::c(courses))
        }
    });
}
