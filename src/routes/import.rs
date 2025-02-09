use axum::{extract::Query, response::IntoResponse};
use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

use common::Schedule;

use crate::common::{self, AppError};

#[derive(Deserialize)]
pub struct Params {
    blob: String,
}

#[instrument(level = "debug")]
pub async fn get(Query(Params { blob }): Query<Params>) -> Result<impl IntoResponse, AppError> {
    let uuid = Uuid::new_v4().to_string();

    Schedule::try_from(blob)
        .map(|s| {
            (
                StatusCode::FOUND,
                [("location", format!("/schedule/{}", uuid))],
                CookieJar::new().add(s.make_cookie(uuid)),
            )
        })
        .map_err(AppError::Anyhow)
}
