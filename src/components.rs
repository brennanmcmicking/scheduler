use maud::{html, Markup};

use crate::middlewares::{Authority, Session};
use crate::components;

pub mod button;
pub mod calendar;
pub mod container;
pub mod courses;
pub mod schedules;
pub mod search_result;

pub fn base(
    content: Markup,
    session: Option<Session>,
) -> Markup {
    let header_right = match session {
        Some(session) => html!(
            div class="grow flex gap-2 justify-end" {
                @match session.authority { 
                    Authority::DISCORD => img src="/assets/discord-mark-white.svg" class="rounded p-1 lg:p-2 bg-[#5865F2]" {},
                    Authority::GOOGLE => img src="/assets/google-g-logo.svg" class="rounded-full lg:p-2 bg-white" {},
                }
                div class="hidden lg:flex items-center" {
                    (session.username)
                }
            }
            a href="/login" class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" {
                "log out"
            }
        ),
        None => html!(
            a href="/login" class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" {
                "log in"
            }
        ),
    };
    html! {
        html class="h-full" {
            head {
                title {"scheduler"}
                script src="/assets/htmx.min.js" {}
                script src="/assets/multi-swap.js" {}
                script src="/assets/tailwind.js" {}
                meta name="viewport" content="width=device-width,initial-scale=1.0" {}
            }
            body hx-ext="multi-swap" class="h-full w-full bg-slate-100 dark:bg-neutral-900 dark:text-white overflow-y-none" {
                div id="app-container" class="h-full flex flex-col" {
                    div class="shadow-lg w-full bg-white dark:bg-neutral-800 block" {
                        div id="header-container" class="max-w-screen-2xl w-full mx-auto lg:px-2 flex max-h-6 lg:max-h-12" {
                            div class="w-1/3 flex justify-start gap-2 lg:py-2" {
                                (components::button::link("/", html!("home")))
                                (components::button::link("/donate", html!("donate")))
                            }
                            div class="w-1/3 flex justify-center items-center" {
                                "uvic scheduler"
                            }
                            div class="w-1/3 flex px-1 lg:py-2 justify-end gap-2" {
                                (header_right)
                            }
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
