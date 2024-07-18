// use hypertext::{html_elements, rsx, GlobalAttributes, Renderable};
use maud::{html, Markup};

pub fn c(content: Markup) -> Markup {
    html! {
        button class="border-2 rounded bg-slate-200 p-1 m-1" hx-post="/clicked" {
            (content)
        }
    }
}
