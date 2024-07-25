use maud::{html, Markup};

pub mod button;
pub mod calendar;
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
            body class="bg-slate-100" {
                div id="app-container" class="lg:px-64 xl:px-96" {
                    (content)
                }
            }
        }
    }
}
