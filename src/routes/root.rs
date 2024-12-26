use axum::extract::State;
use maud::{html, Markup};
use std::sync::Arc;
use tracing::instrument;

use crate::{components, middlewares::{Schedules, Session}};

use super::DatabaseAppState;

#[instrument(level = "debug", skip(state))]
pub async fn root(
    State(state): State<Arc<DatabaseAppState>>, 
    schedules: Schedules,
    session: Option<Session>,
) -> Markup {
    components::base(html! {
        div class="flex flex-col gap-2 py-2 px-2 lg:px-64 h-full justify-items-center" {
            form action="/schedule" method="post" class="flex gap-2" {
                input type="text" id="name" name="name" size="10" placeholder="schedule name" class="p-2 rounded-lg text-black grow border-neutral-400 border-2 dark:border-0" {}
                select name="term" id="term" class="text-black rounded-lg p-2 border-2 border-neutral-400 dark:border-0" {
                    @for term in &state.get_terms() {
                        option value={(term)} {
                            (term.human_display())
                        }
                    }
                }
                button action="submit" class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" {
                    "create"
                }
            }
            (components::schedules::view(schedules.schedules))
        }
    }, session)
}
