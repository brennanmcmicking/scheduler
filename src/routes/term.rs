use crate::{middlewares::CookieUserState, scraper::Term};
use axum::extract::{Extension, Path, State};
use maud::{html, Markup};
use std::sync::Arc;

use crate::components;

use super::DatabaseAppState;

pub async fn term(
    Path(id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
) -> Markup {
    dbg!(&user_state);
    dbg!(&id);

    let term: Term = id.parse().unwrap();


    let courses = state.courses(term);

    components::base(html! {
        (components::container::render(&courses))
    })
}
