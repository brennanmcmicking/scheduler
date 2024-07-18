use maud::{html, Markup};

use crate::components;
// use hypertext::{html_elements, rsx, Renderable, Rendered};

pub fn root() -> Markup {
    return components::base(html! {
        // p { "ptag" }
        // (components::button::c(html! { p {"button"}}))
        div class="flex justify-center gap-4 h-4/5" {
            div id="calendar" class="h-full w-1/2" {}
        }
    });
}
