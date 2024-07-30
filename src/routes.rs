use std::{env::current_dir, path::PathBuf, sync::Arc};

use anyhow::{Ok, Result};

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use regex::bytes::Regex;
use rusqlite::{Connection, OpenFlags};
use tokio::fs;
use tower_http::services::ServeDir;

use crate::{middlewares, scraper::Term};

mod calendar;
mod root;
mod search;

pub trait AppState {
    fn courses(&self) -> Vec<String>;
}

#[derive(Clone)]
pub struct RegularAppState {
    pub courses: Vec<String>,
}

impl AppState for RegularAppState {
    fn courses(&self) -> Vec<String> {
        self.courses.clone()
    }
}

pub struct DatabaseAppState {
    dir: PathBuf,
}

impl DatabaseAppState {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub async fn get_terms(&self) -> Result<Vec<Term>> {
        let mut terms = Vec::new();

        let mut entries = fs::read_dir(&self.dir).await?;

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
            terms.push(term);
        }
        terms.sort();
        terms.reverse();

        Ok(terms)
    }

    pub fn get_conn(&self, term: &Term) -> Option<Connection> {
        let name = format!("sections_{}.sqlite3", term);
        Connection::open_with_flags(name, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
    }
}

impl AppState for Arc<DatabaseAppState> {
    fn courses(&self) -> Vec<String> {
        // FIXME: pass in the term
        let term = "202409".parse().unwrap();
        let Some(conn) = self.get_conn(&term) else {
            return Vec::new();
        };
        let mut stmt = conn
            .prepare("SELECT subject_code, course_code FROM section LIMIT 50")
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
}

pub fn make_app(_courses: Vec<String>) -> Router {
    type State = Arc<DatabaseAppState>;

    let state: State = Arc::new(DatabaseAppState::new(
        current_dir().expect("couldn't access current directory"),
    ));

    let calendar_route = Router::new()
        .route("/", put(calendar::add_to_calendar::<State>))
        .route("/", delete(calendar::rm_from_calendar::<State>));

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root::<State>))
        .route("/search", post(search::search::<State>))
        .nest("/calendar", calendar_route)
        .with_state(state)
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(axum::middleware::from_fn(middlewares::parse_cookie)),
        )
}
