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
    routes::{DatabaseAppState, SectionType},
    scraper::{Section, Term, ThinCourse, ThinSection},
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

impl Default for Selection {
    fn default() -> Self {
        Self {
            lecture: ThinSection {
                crn: 0,
            },
            lab: None,
            tutorial: None,
        }
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
            .permanent()
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

impl From<Vec<Section>> for SelectedCourses {
    fn from(value: Vec<Section>) -> Self {
        let mut courses: BTreeMap<ThinCourse, Selection> = BTreeMap::new();
        value.iter().for_each(|section| {
            let thin_course = ThinCourse {
                subject_code: section.subject_code.clone(),
                course_code: section.course_code.clone(),
            };

            let mut current: Selection;
            if courses.contains_key(&thin_course) {
                current = courses.get_mut(&thin_course).unwrap().clone();
            } else {
                current = Selection::default();
            }

            let thin_section = ThinSection {
                crn: section.crn
            };

            match section.get_type() {
                SectionType::Lecture => current.lecture = thin_section,
                SectionType::Lab => current.lab = Some(thin_section),
                SectionType::Tutorial => current.tutorial = Some(thin_section),
            }
            courses.insert(thin_course, current);
        });

        SelectedCourses {
            courses,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Authority {
    DISCORD,
    GOOGLE,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub user_id: String,
    pub username: String,
    pub authority: Authority,
    pub session_id: String,
}

impl Session {
    pub fn to_base64(&self) -> String {
        let json = serde_json::to_string(&self).expect("failed to serialize to json");
        STANDARD_NO_PAD.encode(json)
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
                        match state.is_valid_session(&sess.user_id, &sess.session_id).await {
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

#[derive(Deserialize)]
struct SchedulePath {
    schedule_id: String,
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
            Err(e) => {
                match e {
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
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleWithId {
    pub schedule: Schedule,
    pub id: String,
}

impl ScheduleWithId {
    pub fn to_base64(&self) -> String {
        let userstate_json = serde_json::to_string(&self).expect("failed to serialize to json");
        STANDARD_NO_PAD.encode(userstate_json)
    }
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
            Err(e) => {
                match e {
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
                }
            }
        }
    }
}

pub struct GoogleCsrfCookie {
    pub value: String,
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