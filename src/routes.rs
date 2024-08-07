use std::{collections::HashMap, env::current_dir, ops::DerefMut, path::PathBuf, sync::Arc};

use anyhow::{Context, Ok, Result};

use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Router,
};
use r2d2_sqlite::SqliteConnectionManager;
use regex::bytes::Regex;
use rusqlite::{Connection, OpenFlags};
use tokio::fs;
use tower_http::{
    services::ServeDir,
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{debug_span, error};

use crate::{
    middlewares,
    scraper::{Term, ThinCourse},
};

mod calendar;
mod root;
mod search;
mod term;

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

    fn courses(&self, term: Term) -> Result<Vec<ThinCourse>> {
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
        .route("/term/:id", get(term::term))
        .route("/term/:id/search", post(search::search))
        .route(
            "/term/:id/calendar",
            put(calendar::add_to_calendar).delete(calendar::rm_from_calendar),
        )
        .with_state(state)
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(axum::middleware::from_fn(middlewares::parse_cookie)),
        )
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
