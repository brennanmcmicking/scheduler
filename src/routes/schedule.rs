use crate::{components::schedules, middlewares::{Schedule, Schedules, SelectedCourses}, routes::selected_sections, scraper::Term};
use axum::{debug_handler, extract::{Path, Request, State}, middleware::Next, response::IntoResponse};
use axum_extra::extract::{cookie::Cookie, CookieJar, Form};
use maud::{html, Markup};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::components;

use super::{AppError, DatabaseAppState};

#[instrument(level = "debug", skip(state))]
pub async fn get(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    schedule: Schedule,
) -> Result<Markup, AppError> {
    debug!("schedule endpoint called");
    let search_courses = state.thin_courses(schedule.term)?;
    let courses = state.courses(schedule.term, &schedule.selected.thin_courses())?;
    let sections = selected_sections(&courses, &schedule.selected);

    Ok(components::base(html! {
        (components::container::main_container(&schedule_id, &search_courses, &courses, &sections))
    }))
}

#[derive(Clone, Debug, Deserialize)]
pub struct Create {
    term: String,
    name: String,
}


#[instrument(level = "debug", skip(state))]
#[debug_handler]
pub async fn post(
    State(state): State<Arc<DatabaseAppState>>,
    Form(Create { term, name }): Form<Create>,
) -> Result<impl IntoResponse, AppError> {
    let uuid = Uuid::new_v4();
    let term: Term = term.parse().unwrap();
    if state.get_terms().contains(&term) {
        Ok((
            CookieJar::new().add(Schedule {
                name,
                term,
                selected: SelectedCourses::default(),
            }.make_cookie(uuid.to_string())),
            [("location", format!("/schedule/{}", uuid))],
            StatusCode::MOVED_PERMANENTLY
        ))
    } else {
        Err(AppError::Code(StatusCode::BAD_REQUEST))
    }
}

#[instrument(level = "debug")]
pub async fn delete(
    Path(schedule_id): Path<String>,
    schedules: Schedules,
) -> Result<impl IntoResponse, AppError> {
    let new_schedules = schedules.schedules.into_iter().filter(|s| !s.id.eq(&schedule_id)).collect();
    let mut cookie = Cookie::build((schedule_id, "")).path("/").build();
    cookie.make_removal();
    Ok((
        CookieJar::new().add(
            cookie
        ),
        schedules::view(new_schedules)
    ))
}

#[instrument(level = "debug")]
pub async fn not_found(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, AppError> {
    let res = next.run(req).await;
    if res.status() == StatusCode::NOT_FOUND {
        return Ok(components::base(html! {
            div class="h-full flex items-center justify-center" {
                "That schedule could not be found."
            }
        }).into_response());
    }

    Ok(res)
}