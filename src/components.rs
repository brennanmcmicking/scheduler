use maud::{html, Markup};

pub mod button;

pub fn base(content: Markup) -> Markup {
    html! {
        html {
            head {
                title {"scheduler"}
                script src="https://unpkg.com/htmx.org@2.0.1" {}
                script src="https://cdn.tailwindcss.com" {}
                link rel="stylesheet" href="https://uicdn.toast.com/calendar/latest/toastui-calendar.min.css" {}
                script src="https://uicdn.toast.com/calendar/latest/toastui-calendar.min.js" {}
                script src="/assets/calendar.js" defer {}
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
