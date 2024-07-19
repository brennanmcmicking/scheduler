use maud::{html, Markup};

pub mod button;
pub mod searchResult;

pub fn base(content: Markup) -> Markup {
    html! {
        html {
            head {
                title {"scheduler"}
                script src="/assets/htmx.min.js" {}
                script src="/assets/tailwind.js" {}
            }
            body class="bg-slate-100" {
            div class="w-full bg-slate-800 flex justify-center gap-4" {
                div class="bg-slate-400 rounded p-1 my-1" { "scheduler" }
            }
                (content)
            }
        }
    }
}
