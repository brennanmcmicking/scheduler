use std::{collections::BTreeMap, str};

use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::{request, StatusCode},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use serde::{Deserialize, Serialize};

use crate::scraper::{Term, ThinCourse, ThinSection};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SelectedCourses {
    pub courses: BTreeMap<ThinCourse, Vec<ThinSection>>,
}

impl SelectedCourses {
    pub fn thin_courses(&self) -> Vec<&ThinCourse> {
        self.courses.keys().collect()
    }

    fn cookie_name(term: Term) -> String {
        format!("selected_courses_{}", term)
    }

    pub fn make_cookie(&self, term: Term) -> Cookie<'static> {
        let userstate_json = serde_json::to_string(&self).expect("failed to serialize to json");

        let userstate_base64 = STANDARD_NO_PAD.encode(userstate_json);

        Cookie::build((Self::cookie_name(term), userstate_base64))
            .http_only(true)
            .secure(true)
            // .max_age(Duration::MAX) // do we want exp date?
            // .domain(value) // TODO: set domain?
            .build()
    }
}

impl<'a> TryFrom<&Cookie<'a>> for SelectedCourses {
    type Error = anyhow::Error;

    fn try_from(cookie: &Cookie<'a>) -> Result<Self, Self::Error> {
        let cookie_base64 = cookie.value();
        let cookie_json = STANDARD_NO_PAD.decode(cookie_base64)?;
        let userstate = serde_json::from_slice(&cookie_json)?;
        Ok(userstate)
    }
}

#[derive(Deserialize)]
struct TermPath {
    term: Term,
}

#[async_trait] // needed to prevent lifetime errors
impl<S: Send + Sync> FromRequestParts<S> for SelectedCourses {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(TermPath { term }) =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|err| {
                    tracing::trace!("failed to get term from path: {:?}", err);
                    StatusCode::NOT_FOUND
                })?;

        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();

        Ok(match jar.get(&Self::cookie_name(term)) {
            Some(cookie) => Self::try_from(cookie).map_err(|err| {
                tracing::trace!("failed to parse: {:?}", err);
                StatusCode::NOT_FOUND
            })?,

            None => Self::default(),
        })
    }
}
