use core::str;
use std::collections::BTreeMap;

use anyhow::Result;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::Cookie;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};

use crate::scraper::{Course, Section, Term, ThinCourse, ThinSection};

pub enum AppError {
    Anyhow(anyhow::Error),
    Code(StatusCode),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Anyhow(error) => {
                let backtrace = error.backtrace();
                error!(%error, %backtrace);
                AppError::Code(StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
            AppError::Code(c) => (c, c.to_string()).into_response(),
        }
    }
}

impl From<StatusCode> for AppError {
    fn from(err: StatusCode) -> Self {
        Self::Code(err)
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Anyhow(err)
    }
}

#[derive(Clone)]
pub enum Stage {
    LOCAL,
    PROD,
}

impl From<String> for Stage {
    fn from(value: String) -> Self {
        match value.as_str() {
            "prod" => Stage::PROD,
            "local" => Stage::LOCAL,
            _ => Stage::LOCAL,
        }
    }
}

pub enum SectionType {
    Lecture,
    Lab,
    Tutorial,
}

impl From<String> for SectionType {
    fn from(sequence_code: String) -> Self {
        match sequence_code.get(0..1).unwrap() {
            "A" => SectionType::Lecture,
            "B" => SectionType::Lab,
            "T" => SectionType::Tutorial,
            unparsable_sequence_code => {
                debug!(unparsable_sequence_code);
                unreachable!()
            }
        }
    }
}

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
            lecture: ThinSection { crn: 0 },
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

            let thin_section = ThinSection { crn: section.crn };

            match section.get_type() {
                SectionType::Lecture => current.lecture = thin_section,
                SectionType::Lab => current.lab = Some(thin_section),
                SectionType::Tutorial => current.tutorial = Some(thin_section),
            }
            courses.insert(thin_course, current);
        });

        SelectedCourses { courses }
    }
}

#[derive(Deserialize)]
pub struct SchedulePath {
    pub schedule_id: String,
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

pub fn selected_sections(courses: &[Course], selected: &SelectedCourses) -> Vec<Section> {
    let crns = selected.crns();
    courses
        .iter()
        .flat_map(|c| c.sections.clone())
        .filter(|s| crns.contains(&s.crn))
        .collect()
}
