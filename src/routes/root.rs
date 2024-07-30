use crate::middlewares::CookieUserState;
use axum::extract::{Extension, State};
use maud::{html, Markup};
use std::sync::Arc;

use crate::components;

use super::DatabaseAppState;

pub async fn root(
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
) -> Markup {
    dbg!(&user_state);
    // dbg!(&state);

    components::base(html! {
        div class="flex flex-col gap-2 p-2 h-full justify-items-center" {
            div class="h-full w-full text-white grow rounded-lg bg-neutral-800 flex justify-center items-center" {
                "select a term"
            }
            @for term in &state.get_terms() {
                a class="h-full w-full text-white grow rounded-lg bg-blue-900 hover:bg-blue-800 flex justify-center items-center" href={ "/term/" (term) } {
                    (term)
                }
            }
        }
    })
}
