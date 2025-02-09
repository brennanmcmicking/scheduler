use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path},
    http::{request, StatusCode},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use std::sync::Arc;
use tracing::debug;

use crate::{
    common::{Schedule, SchedulePath, ScheduleWithId, Schedules},
    data::{auth::GoogleCsrfCookie, store::Session, DatabaseAppState},
};

impl<'a> TryFrom<&Cookie<'a>> for Schedule {
    type Error = anyhow::Error;

    fn try_from(cookie: &Cookie<'a>) -> Result<Self, Self::Error> {
        debug!("trying to decode cookie");
        let cookie_base64 = cookie.value().to_string();
        Schedule::try_from(cookie_base64)
    }
}

#[async_trait] // needed to prevent lifetime errors
impl<S> FromRequestParts<S> for Schedule
where
    Arc<DatabaseAppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(SchedulePath { schedule_id }) = Path::from_request_parts(parts, state)
            .await
            .map_err(|err| {
                tracing::trace!("failed to get schedule from path: {:?}", err);
                StatusCode::NOT_FOUND
            })?;

        let session = Session::from_request_parts(parts, state).await;

        debug!("schedule_id: {}", schedule_id);
        // state.get_user_schedule("asdf", schedule_id.as_str()).await.map_err(|_e| StatusCode::NOT_FOUND)
        match session {
            Ok(session) => {
                let state = Arc::from_ref(state);
                state
                    .get_user_schedule(&session.user_id, schedule_id.as_str())
                    .await
                    .map_err(|_e| StatusCode::NOT_FOUND)
            }
            Err(e) => match e {
                StatusCode::UNAUTHORIZED => Err(StatusCode::UNAUTHORIZED),
                _ => {
                    let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
                    match jar.get(&schedule_id) {
                        Some(cookie) => Ok(Schedule::try_from(cookie).map_err(|err| {
                            tracing::trace!("failed to parse: {:?}", err);
                            debug!("failed to parse: {:?}", err);
                            StatusCode::NOT_FOUND
                        })?),
                        None => Err(StatusCode::NOT_FOUND),
                    }
                }
            },
        }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Session
where
    Arc<DatabaseAppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        debug!("trying to decode session cookie");
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
        match jar.get("session") {
            Some(cookie) => {
                debug!("{}", cookie.value());
                let session_json = STANDARD_NO_PAD.decode(cookie.value()).map_err(|e| {
                    debug!("could not base64-decode session {}, {}", cookie.value(), e);
                    StatusCode::BAD_REQUEST
                })?;
                let result: Result<Session, _> = serde_json::from_slice(&session_json);
                match result.ok() {
                    Some(sess) => {
                        let state = Arc::from_ref(state);
                        match state
                            .is_valid_session(&sess.user_id, &sess.session_id)
                            .await
                        {
                            true => Ok(sess),
                            false => Err(StatusCode::UNAUTHORIZED),
                        }
                    }
                    None => Err(StatusCode::UNAUTHORIZED),
                }
            }
            None => Err(StatusCode::NOT_FOUND),
        }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Schedules
where
    Arc<DatabaseAppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await;

        match session {
            Ok(session) => {
                let state = Arc::from_ref(state);
                let user = state
                    .get_user(&session.user_id)
                    .await
                    .map_err(|_e| StatusCode::NOT_FOUND)?;
                Ok(Schedules {
                    schedules: user.schedules,
                })
            }
            Err(e) => match e {
                StatusCode::UNAUTHORIZED => Err(StatusCode::UNAUTHORIZED),
                _ => {
                    let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
                    Ok(Schedules {
                        schedules: jar
                            .iter()
                            .filter_map(|cookie| match Schedule::try_from(cookie) {
                                Ok(schedule) => Some(ScheduleWithId {
                                    schedule,
                                    id: cookie.name().to_string(),
                                }),
                                Err(_e) => None,
                            })
                            .collect(),
                    })
                }
            },
        }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for GoogleCsrfCookie {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
        let csrf_cookie = jar.get("g_csrf_token").ok_or(StatusCode::BAD_REQUEST)?;
        Ok(GoogleCsrfCookie {
            value: csrf_cookie.value().to_owned(),
        })
    }
}
