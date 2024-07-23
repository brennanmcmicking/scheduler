use maud::{html, Markup};

pub fn c(courses: &Vec<&String>) -> Markup {
    return html! {
        div {
            @for course in courses {
                form class="flex border-b justify-between items-center" {
                    div class="" {
                        (course)
                    }
                    button name="course" value=(course) class="bg-green-500 rounded-lg h-full p-1 my-1 text-xs" hx-put="/calendar" hx-target="#calendar-view" {
                        "add"
                    }
                }
            }
        }
    };
}
