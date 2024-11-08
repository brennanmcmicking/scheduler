use axum::extract::State;
use maud::{html, Markup};
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::components;

use super::DatabaseAppState;

#[instrument(level = "debug", skip(state))]
pub async fn root(State(state): State<Arc<DatabaseAppState>>) -> Markup {
    debug!("health");
    html!(200)
}
