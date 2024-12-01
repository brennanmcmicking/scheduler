use maud::{html, Markup};

pub mod button;
pub mod calendar;
pub mod container;
pub mod search_result;
pub mod schedules;
pub mod courses;

pub fn base(content: Markup) -> Markup {
    html! {
        html class="h-full" {
            head {
                title {"scheduler"}
                script src="/assets/htmx.min.js" {}
                script src="/assets/multi-swap.js" {}
                script src="/assets/tailwind.js" {}
                meta name="viewport" content="width=device-width,initial-scale=1.0" {}
            }
            body hx-ext="multi-swap" class="w-full bg-slate-100 dark:bg-neutral-900 dark:text-white overflow-y-none" {
                div id="app-container" class="flex flex-col h-screen" {
                    div class="shadow-lg w-full bg-white dark:bg-neutral-800 block" {
                        div id="header-container" class="max-w-screen-2xl w-full mx-auto flex" {
                            div class="w-1/3 px-1 lg:py-2" {
                                a href="/" class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" {
                                    "home"
                                }
                            }
                            div class="w-1/3 flex justify-center items-center" {
                                "uvic scheduler"
                            }
                            div class="w-1/3" {}
                        }
                    }
                    div id="base-container" class="mx-auto w-full h-5/6 grow max-w-screen-2xl block" {
                        (content)
                    }
                }
            }
        }
    }
}
