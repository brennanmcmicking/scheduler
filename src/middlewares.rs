use std::{
    collections::{BTreeMap, HashMap},
    str::{self},
    sync::Arc,
};

use aws_sdk_dynamodb::types::AttributeValue;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path},
    http::{request, StatusCode},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use tracing::debug;

use anyhow::anyhow;

use crate::{
    routes::DatabaseAppState,
    scraper::{Term, ThinCourse, ThinSection},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub lecture: ThinSection,
    pub lab: Option<ThinSection>,
    pub tutorial: Option<ThinSection>,
}

impl Selection {
    pub fn crns(&self) -> Vec<u64> {
        [
            Some(self.lecture.clone()),
            self.lab.clone(),
            self.tutorial.clone(),
        ]
        .iter()
        .filter_map(Option::as_ref)
        .map(|i| i.crn)
        .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schedule {
    pub name: String,
    pub term: Term,
    pub selected: SelectedCourses,
}

impl Schedule {
    pub fn to_base64(&self) -> String {
        let userstate_json = serde_json::to_string(&self).expect("failed to serialize to json");
        STANDARD_NO_PAD.encode(userstate_json)
    }

    pub fn make_cookie(&self, id: String) -> Cookie<'static> {
        let userstate_base64 = self.to_base64();
        Cookie::build((id, userstate_base64))
            .http_only(true)
            .secure(true)
            .path("/")
            // .max_age(Duration::MAX) // do we want exp date?
            // .domain(value) // TODO: set domain?
            .build()
    }
}

impl TryFrom<String> for Schedule {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let schedule_json = STANDARD_NO_PAD.decode(value)?;
        debug!("{}", str::from_utf8(&schedule_json)?);
        let userstate = serde_json::from_slice(&schedule_json)?;
        Ok(userstate)
    }
}

impl<'a> TryFrom<&Cookie<'a>> for Schedule {
    type Error = anyhow::Error;

    fn try_from(cookie: &Cookie<'a>) -> Result<Self, Self::Error> {
        debug!("trying to decode cookie");
        let cookie_base64 = cookie.value().to_string();
        Schedule::try_from(cookie_base64)
    }
}

impl TryFrom<&HashMap<String, AttributeValue>> for Schedule {
    type Error = anyhow::Error;

    fn try_from(value: &HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
        let raw = value
            .get("schedule")
            .ok_or(anyhow!("failed to get schedule value from attribute map"))?
            .as_s()
            .map_err(|_e| anyhow!("could not parse schedule attribute value to string"))?;

        Ok(serde_json::from_str(raw)?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SelectedCourses {
    pub courses: BTreeMap<ThinCourse, Selection>,
}

impl SelectedCourses {
    pub fn thin_courses(&self) -> Vec<&ThinCourse> {
        self.courses.keys().collect()
    }

    pub fn crns(&self) -> Vec<u64> {
        self.courses.values().flat_map(|s| s.crns()).collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub user_id: String,
    pub session_id: String,
}

impl Session {
    pub async fn is_validate(&self, state: &Arc<DatabaseAppState>) -> bool {
        state
            .is_valid_session(&self.user_id, &self.session_id)
            .await
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
                let result: Result<Session, _> = serde_json::from_str(cookie.value());
                match result.ok() {
                    Some(sess) => {
                        let state = Arc::from_ref(state);
                        match sess.is_validate(&state).await {
                            true => Ok(sess),
                            false => Err(StatusCode::BAD_REQUEST),
                        }
                    }
                    None => Err(StatusCode::BAD_REQUEST),
                }
            }
            None => Err(StatusCode::NOT_FOUND),
        }
    }
}

#[derive(Deserialize)]
struct SchedulePath {
    schedule_id: String,
}

#[async_trait] // needed to prevent lifetime errors
impl<S: Send + Sync> FromRequestParts<S> for Schedule
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
            Err(_e) => {
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleWithId {
    pub schedule: Schedule,
    pub id: String,
}

#[derive(Debug)]
pub struct Schedules {
    pub schedules: Vec<ScheduleWithId>,
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
            Err(_e) => {
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
        }
    }
}
