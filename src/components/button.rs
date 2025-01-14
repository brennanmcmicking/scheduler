use maud::{html, Markup};

pub fn form(post_target: &str, form_children: Markup, button_text: &str) -> Markup {
    html! {
        form class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 transition rounded-lg h-full p-1 my-1 text-xl" hx-post=(post_target) {
            (form_children)
            button class="" {
                (button_text)
            }
        }
    }
}

pub fn link(destination: &str, children: Markup) -> Markup {
    html! {
        a href=(destination) class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" {
            (children)
        }
    }
}