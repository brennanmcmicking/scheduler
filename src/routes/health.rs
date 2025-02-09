use axum::extract::State;
use maud::{html, Markup};
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::data::DatabaseAppState;

#[instrument(level = "debug", skip(_state))]
pub async fn root(State(_state): State<Arc<DatabaseAppState>>) -> Markup {
    debug!("health");
    html!("200")
}
