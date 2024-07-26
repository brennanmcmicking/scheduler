use maud::{html, Markup};

pub fn c(courses: &Vec<String>) -> Markup {
    return html! {
        div {
            @for course in courses {
                form class="flex border-b dark:border-neutral-400 justify-between items-center mb-0" {
                    div class="text-xl dark:text-white" {
                        (course)
                    }
                    button name="course" value=(course)
                    class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 text-black dark:text-white rounded-lg h-full p-1 my-1 text-xl shadow-lg"
                    hx-put="/calendar" hx-target="#calendar-container" hx-swap="outerHTML" {
                        "add"
                    }
                }
            }
        }
    };
}
