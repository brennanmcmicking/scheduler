use std::{collections::HashMap, env::current_dir, ops::DerefMut, path::PathBuf, sync::Arc};

use anyhow::{Ok, Result};

use axum::{
    http::Request,
    routing::{delete, get, post, put},
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
use tracing::debug_span;

use crate::{middlewares, scraper::Term};

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

    fn courses(&self, term: Term) -> Vec<String> {
        let Some(conn) = self.get_conn(&term) else {
            return Vec::new();
        };
        let mut stmt = conn
            .prepare("SELECT subject_code, course_code FROM section")
            .unwrap();

        let mut courses = Vec::new();
        let mut rows = stmt.query(()).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let subject: String = row.get_unwrap("subject_code");
            let course: String = row.get_unwrap("course_code");
            courses.push(format!("{subject} {course}"));
        }
        courses
    }

    pub fn get_conn(&self, term: &Term) -> Option<impl DerefMut<Target = Connection>> {
        self.terms.get(term).and_then(|p| p.get().ok())
    }
}

pub async fn make_app() -> Router {
    type State = Arc<DatabaseAppState>;

    let state: State = Arc::new(
        DatabaseAppState::new(current_dir().expect("couldn't access current directory"))
            .await
            .expect("failed to initialize database state"),
    );

    let calendar_route = Router::new()
        .route("/", put(calendar::add_to_calendar))
        .route("/", delete(calendar::rm_from_calendar));

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root))
        .route("/term/:id", get(term::term))
        .route("/search", post(search::search))
        .nest("/calendar", calendar_route)
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
