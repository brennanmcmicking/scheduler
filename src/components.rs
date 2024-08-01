use maud::{html, Markup};

pub mod button;
pub mod calendar;
pub mod container;
pub mod search_result;

pub fn base(content: Markup) -> Markup {
    html! {
        html {
            head {
                title {"scheduler"}
                script src="/assets/htmx.min.js" {}
                script src="/assets/tailwind.js" {}
                meta name="viewport" content="width=device-width,initial-scale=1.0" {}
            }
            body class="bg-slate-100 dark:bg-neutral-900" {
                div id="app-container" class="contents" {
                    (content)
                }
            }
        }
    }
}
