use crate::{middlewares::CookieUserState, scraper::Term};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
};
use maud::{html, Markup};
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::components;

use super::{AppError, DatabaseAppState};

#[instrument(level = "debug", skip(state))]
pub async fn term(
    Path(id): Path<String>, // TODO: implement Deserialize on Term
    State(state): State<Arc<DatabaseAppState>>,
    Extension(user_state): CookieUserState,
) -> Result<Markup, AppError> {
    debug!("term");
    let term = id.parse::<Term>().map_err(|_| StatusCode::BAD_REQUEST)?;

    let courses = state.courses(term)?;

    Ok(components::base(html! {
        (components::container::calendar_container(term, &courses))
    }))
}
