use maud::{html, Markup};

pub fn c(courses: Vec<String>) -> Markup {
    return html! {
        div {
            @for course in &courses {
                div { (course) }
            }
        }
    };
}
