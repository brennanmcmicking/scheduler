use crate::{components::schedules, middlewares::{Schedule, Schedules, SelectedCourses, Session}, routes::selected_sections, scraper::Term};
use axum::{debug_middleware, extract::{Path, Request, State}, middleware::Next, response::IntoResponse};
use axum_extra::extract::{cookie::Cookie, CookieJar, Form};
use maud::{html, Markup};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;
use std::sync::Arc;
use tracing::instrument;

use crate::components;

use super::{AppError, DatabaseAppState};

#[instrument(level = "debug", skip(state))]
pub async fn get(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    schedule: Schedule,
    session: Option<Session>,
) -> Result<Markup, AppError> {
    let search_courses = state.thin_courses(schedule.term)?;
    let courses = state.courses(schedule.term, &schedule.selected.thin_courses())?;
    let sections = selected_sections(&courses, &schedule.selected);

    Ok(components::base(html! {
        (components::container::main_container(&schedule_id, &search_courses, &courses, &sections))
    }, session))
}

#[derive(Clone, Debug, Deserialize)]
pub struct Create {
    term: String,
    name: String,
}


#[instrument(level = "debug", skip(state))]
pub async fn post(
    State(state): State<Arc<DatabaseAppState>>,
    session: Option<Session>,
    Form(Create { term, name }): Form<Create>,
) -> Result<impl IntoResponse, AppError> {
    let uuid = Uuid::new_v4();
    let term: Term = term.parse().unwrap();
    let name = match name.len() > 100 {
        true => name[..100].to_string(),
        false => name,
    };
    let new_schedule = Schedule {
        name,
        term,
        selected: SelectedCourses::default()
    };
    if state.get_terms().contains(&term) {
        let jar = match session {
            Some(sess) => {
                let _ = state.set_user_schedule(&sess.user_id, &uuid.to_string(), &new_schedule).await;
                CookieJar::new()
            },
            None => CookieJar::new().add(new_schedule.make_cookie(uuid.to_string())),
        };

        Ok((
            jar,
            [("location", format!("/schedule/{}", uuid))],
            StatusCode::SEE_OTHER,
        ))
    } else {
        Err(AppError::Code(StatusCode::BAD_REQUEST))
    }
}

#[instrument(level = "debug", skip(state))]
pub async fn delete(
    State(state): State<Arc<DatabaseAppState>>,
    Path(schedule_id): Path<String>,
    session: Option<Session>,
    schedules: Schedules,
) -> Result<impl IntoResponse, AppError> {
    let new_schedules = schedules.schedules.into_iter().filter(|s| !s.id.eq(&schedule_id)).collect();
    let mut jar = CookieJar::new();

    match session {
        Some(sess) => {
            state.delete_user_schedule(&sess.user_id, &schedule_id).await;
        },
        None => { 
            jar = jar.add(
                Cookie::build((schedule_id, ""))
                .path("/")
                .removal()
                .build()
            );
        }
    };

    Ok((
        jar,
        schedules::view(new_schedules)
    ))
}

#[instrument(level = "debug", skip(_state))]
#[debug_middleware]
pub async fn not_found(
    State(_state): State<Arc<DatabaseAppState>>,
    session: Option<Session>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, AppError> {
    let res = next.run(req).await;
    if res.status() == StatusCode::NOT_FOUND {
        return Ok(components::base(html! {
            div class="h-full flex items-center justify-center" {
                "That schedule could not be found."
            }
        }, session).into_response());
    }

    Ok(res)
}