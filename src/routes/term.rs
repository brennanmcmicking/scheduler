use crate::{middlewares::CookieUserState, scraper::Term};
use axum::extract::{Extension, Path, State};
use maud::{html, Markup};
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::components;

use super::DatabaseAppState;

#[instrument(level = "debug", skip(state))]
pub async fn term(
    Path(id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
) -> Markup {
    debug!("term");
    let term: Term = id.parse().unwrap();

    let courses = state.courses(term);

    components::base(html! {
        (components::container::render(&courses))
    })
}
