use maud::{html, Markup};

pub fn view(content: Markup) -> Markup {
    html! {
        button class="border-2 rounded bg-slate-200 dark:bg-slate-700 p-1 m-1" hx-post="/clicked" {
            (content)
        }
    }
}
