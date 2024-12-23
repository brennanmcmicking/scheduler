use maud::{html, Markup};

pub fn form(post_target: &str, children: Markup) -> Markup {
    html! {
        button class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" hx-post=(post_target) {
            (children)
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