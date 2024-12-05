use std::{collections::BTreeMap, str::{self}};

use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::{request, StatusCode},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::scraper::{Term, ThinCourse, ThinSection};

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
        debug!("{}", str::from_utf8(&schedule_json).unwrap());
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

#[derive(Deserialize)]
struct SchedulePath {
    schedule_id: String,
}

#[async_trait] // needed to prevent lifetime errors
impl<S: Send + Sync> FromRequestParts<S> for Schedule {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        debug!("deserializing parts");
        let Path(SchedulePath { schedule_id }) =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|err| {
                    tracing::trace!("failed to get schedule from path: {:?}", err);
                    debug!("failed to parse: {:?}", err);
                    StatusCode::NOT_FOUND
                })?;

        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();

        debug!("schedule_id: {}", schedule_id);

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
impl<S: Send + Sync> FromRequestParts<S> for Schedules {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
        Ok(
            Schedules {
                schedules: jar.iter().filter_map(|cookie| {
                    match Schedule::try_from(cookie) {
                        Ok(schedule) => Some(ScheduleWithId {
                            schedule,
                            id: cookie.name().to_string()
                        }),
                        Err(_e) => None
                    }
                })
                .collect()
        }
        )
    }
}