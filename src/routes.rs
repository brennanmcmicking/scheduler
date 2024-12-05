use std::{collections::HashMap, env::current_dir, ops::DerefMut, path::PathBuf, sync::Arc};

use anyhow::{Context, Ok, Result};

use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Router,
};
use r2d2_sqlite::SqliteConnectionManager;
use regex::bytes::Regex;
use rusqlite::{params, Connection, OpenFlags};
use tokio::fs;
use tower_http::{
    services::ServeDir,
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{debug, debug_span, error};

use crate::{
    middlewares::{SelectedCourses, Selection},
    scraper::{Course, Days, MeetingTime, Section, Term, ThinCourse, ThinSection},
};

mod calendar;
mod preview;
mod root;
mod search;
mod schedule;
mod share;
mod import;

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

pub fn selected_sections(courses: &[Course], selected: &SelectedCourses) -> Vec<Section> {
    let crns = selected.crns();
    courses
        .iter()
        .flat_map(|c| c.sections.clone())
        .filter(|s| crns.contains(&s.crn))
        .collect()
}

pub struct DatabaseAppState {
    terms: HashMap<Term, r2d2::Pool<SqliteConnectionManager>>,
}

impl DatabaseAppState {
    pub async fn new(dir: PathBuf) -> Result<Self> {
        let mut terms = HashMap::new();

        let mut entries = fs::read_dir(dir).await?;

        let pattern = Regex::new(r"^sections_([0-9]{6})\.sqlite3$")?;

        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let Some(captures) = pattern.captures(file_name.as_encoded_bytes()) else {
                continue;
            };
            let Some(term) = captures.get(1) else {
                continue;
            };

            let term: Term = std::str::from_utf8(term.as_bytes())
                .expect("term numbers should be ascii")
                .parse()?;

            let manager = SqliteConnectionManager::file(file_name)
                .with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY);
            let pool = r2d2::Pool::new(manager)?;

            terms.insert(term, pool);
        }
        Ok(Self { terms })
    }

    pub fn get_terms(&self) -> Vec<Term> {
        let mut terms: Vec<_> = self.terms.keys().cloned().collect();
        terms.sort();
        terms.reverse();

        terms
    }

    fn courses(&self, term: Term, keys: &[&ThinCourse]) -> Result<Vec<Course>> {
        let Some(conn) = self.get_conn(&term) else {
            return Ok(Vec::new());
        };

        let courses = keys.iter().map(|&ThinCourse {subject_code, course_code}| {
            conn.query_row_and_then("
                    SELECT title, campus
                    FROM course
                    WHERE subject_code = ?1 AND course_code = ?2
                ",
                (subject_code, course_code), |row| {
                    // no N+1 problem when it's in memory
                    let sections = conn.prepare("
                            SELECT crn, sequence_code, enrollment, enrollment_capacity, waitlist, waitlist_capacity
                            FROM section
                            WHERE subject_code = ?1 AND course_code = ?2
                        ")?
                        .query_and_then((subject_code, course_code), |row| {
                            let crn = row.get("crn")?;

                            let meeting_times = conn.prepare("
                                SELECT
                                    start_time, end_time, start_date, end_date,
                                    monday, tuesday, wednesday, thursday, friday, saturday, sunday,
                                    building, room
                                FROM meeting_time
                                WHERE crn = ?1
                                ")?
                                .query_and_then((crn,), |row| {
                                    let start_time: Option<String> = row.get("start_time")?;
                                    let end_time: Option<String> = row.get("end_time")?;

                                    let start_date: String = row.get("start_date")?;
                                    let end_date: String = row.get("end_date")?;

                                    Ok(MeetingTime {
                                        start_time: start_time.map(|s| s.parse()).transpose()?,
                                        end_time: end_time.map(|s| s.parse()).transpose()?,
                                        start_date: start_date.parse()?,
                                        end_date: end_date.parse()?,

                                        days: Days {
                                            monday: row.get("monday")?,
                                            tuesday: row.get("tuesday")?,
                                            wednesday: row.get("wednesday")?,
                                            thursday: row.get("thursday")?,
                                            friday: row.get("friday")?,
                                            saturday: row.get("saturday")?,
                                            sunday: row.get("sunday")?,
                                        },

                                        building: row.get("building")?,
                                        room: row.get("room")?,
                                    })
                                })?.collect::<Result<Vec<_>>>()?;

                            Ok(Section {
                                crn,
                                subject_code: subject_code.clone(),
                                course_code: course_code.clone(),
                                sequence_code: row.get("sequence_code")?,
                                enrollment: row.get("enrollment")?,
                                enrollment_capacity: row.get("enrollment_capacity")?,
                                waitlist: row.get("waitlist")?,
                                waitlist_capacity: row.get("waitlist_capacity")?,
                                meeting_times,
                            })
                    })?.collect::<Result<Vec<_>>>()?;

                    Ok(Course {
                       subject_code: subject_code.clone(),
                       course_code: course_code.clone(),
                       title: row.get("title")?,
                       campus: row.get("campus")?,
                       sections,
                    })
                })
            }).collect::<Result<Vec<_>>>()?;

        Ok(courses)
    }

    fn thin_courses(&self, term: Term) -> Result<Vec<ThinCourse>> {
        let Some(conn) = self.get_conn(&term) else {
            return Ok(Vec::new());
        };

        let courses = conn
            .prepare("SELECT subject_code, course_code FROM course")
            .context("failed to prepare courses SQL statement")?
            .query_and_then((), |row| {
                let subject_code = row.get("subject_code")?;
                let course_code = row.get("course_code")?;
                Ok(ThinCourse {
                    subject_code,
                    course_code,
                })
            })
            .context("failed to execute query")?
            .collect::<Result<Vec<_>>>() // quit on the first error
            .context("failed to get query rows")?;

        Ok(courses)
    }

    fn search(&self, term: Term, query: &str) -> Result<Vec<ThinCourse>> {
        let db = self
            .get_conn(&term)
            .context("failed to get conn from pool")?;
        let courses = db
            .prepare(
                "SELECT subject_code, course_code
                FROM course
                WHERE subject_code || course_code LIKE '%' || ?1 || '%'
                   OR subject_code || ' ' || course_code LIKE '%' || ?2 || '%'",
            )?
            .query_and_then((query, query), |row| {
                let subject_code = row.get("subject_code")?;
                let course_code = row.get("course_code")?;
                Ok(ThinCourse {
                    subject_code,
                    course_code,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(courses)
    }

    fn default_thin_sections(&self, term: &Term, course: ThinCourse) -> Result<Selection> {
        let conn = self
            .get_conn(term)
            .context("failed to get conn from pool")?;

        let sections = conn.prepare(
            "SELECT sequence_code, crn FROM section WHERE subject_code = ?1 AND course_code = ?2 ORDER BY sequence_code",
        )
        .context("failed to prepare courses SQL statement")?
        .query_and_then((&course.subject_code, &course.course_code), |row| {
            let sequence_code: String = row.get(0)?;
            let crn: u64 = row.get(1)?;
            Ok((sequence_code, ThinSection { crn }))
        })
        .context("query failed")?
        .collect::<anyhow::Result<Vec<_>>>()?;

        let lecture = sections
            .iter()
            .filter(|s| s.0.starts_with("A"))
            .map(|(_, ts)| ts.clone())
            .collect::<Vec<_>>()
            .first()
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "Expected to find lecture section for course {} {}",
                    &course.subject_code, &course.course_code
                )
            });

        let lab = sections
            .iter()
            .filter(|s| s.0.starts_with("B"))
            .map(|(_, ts)| ts.clone())
            .collect::<Vec<_>>()
            .first()
            .cloned();

        let tutorial = sections
            .iter()
            .filter(|s| s.0.starts_with("T"))
            .map(|(_, ts)| ts.clone())
            .collect::<Vec<_>>()
            .first()
            .cloned();

        Ok(Selection {
            lecture,
            lab,
            tutorial,
        })
    }

    fn get_section(&self, term: &Term, section: &ThinSection) -> Result<Section> {
        let db = self
            .get_conn(term)
            .context("failed to get conn from pool")?;

        let meeting_times = db
            .prepare(
                "
            SELECT
                start_time, end_time, start_date, end_date,
                monday, tuesday, wednesday, thursday, friday, saturday, sunday,
                building, room
            FROM meeting_time
            WHERE crn = ?1
            ",
            )?
            .query_and_then((section.crn,), |row| {
                let start_time: Option<String> = row.get("start_time")?;
                let end_time: Option<String> = row.get("end_time")?;

                let start_date: String = row.get("start_date")?;
                let end_date: String = row.get("end_date")?;

                Ok(MeetingTime {
                    start_time: start_time.map(|s| s.parse()).transpose()?,
                    end_time: end_time.map(|s| s.parse()).transpose()?,
                    start_date: start_date.parse()?,
                    end_date: end_date.parse()?,

                    days: Days {
                        monday: row.get("monday")?,
                        tuesday: row.get("tuesday")?,
                        wednesday: row.get("wednesday")?,
                        thursday: row.get("thursday")?,
                        friday: row.get("friday")?,
                        saturday: row.get("saturday")?,
                        sunday: row.get("sunday")?,
                    },

                    building: row.get("building")?,
                    room: row.get("room")?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let result = db
            .prepare(
                "SELECT *
                FROM section
                WHERE crn = :crn
                LIMIT 1",
            )?
            .query_row(params![section.crn], |row| {
                let crn: u64 = row.get("crn")?;
                let subject_code = row.get("subject_code")?;
                let course_code = row.get("course_code")?;
                let sequence_code = row.get("sequence_code")?;
                let enrollment = row.get("enrollment")?;
                let enrollment_capacity = row.get("enrollment_capacity")?;
                let waitlist = row.get("waitlist")?;
                let waitlist_capacity = row.get("waitlist_capacity")?;
                Result::Ok(Section {
                    crn,
                    subject_code,
                    course_code,
                    sequence_code,
                    enrollment,
                    enrollment_capacity,
                    waitlist,
                    waitlist_capacity,
                    meeting_times,
                })
            })?;
        Ok(result)
    }

    pub fn get_conn(&self, term: &Term) -> Option<impl DerefMut<Target = Connection>> {
        self.terms.get(term).and_then(|p| p.get().ok())
    }
}

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

pub async fn make_app() -> Router {
    type State = Arc<DatabaseAppState>;

    let state: State = Arc::new(
        DatabaseAppState::new(current_dir().expect("couldn't access current directory"))
            .await
            .expect("failed to initialize database state"),
    );

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root))
        .route("/share/:schedule_id", get(share::get))
        .route("/import" , get(import::get))
        .route("/schedule", post(schedule::post))
        .nest(
            "/schedule/:schedule_id",
            Router::new()
                .route("/", get(schedule::get))
                .route("/", delete(schedule::delete))
                .route("/search", post(search::search))
                .nest(
                    "/calendar",
                    Router::new()
                        .route("/", get(calendar::get_calendar))
                        .route("/", put(calendar::add_to_calendar))
                        .route("/", patch(calendar::update_calendar))
                        .route("/", delete(calendar::rm_from_calendar))
                        .route("/preview", get(preview::preview)),
                ),
        )
        // TODO: add .fallback() handler
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    debug_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_response(DefaultOnResponse::new().latency_unit(LatencyUnit::Micros)),
        )
}
