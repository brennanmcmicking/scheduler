use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

mod root;
mod search;

pub trait AppState: Clone + Send + Sync + 'static {
    fn courses(&self) -> &Vec<String>;
}

#[derive(Clone)]
pub struct RegularAppState {
    pub courses: Vec<String>,
}

impl AppState for RegularAppState {
    fn courses(&self) -> &Vec<String> {
        &self.courses
    }
}

pub fn make_app(courses: Vec<String>) -> Router {
    let state = RegularAppState { courses: courses };
    return Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root::<RegularAppState>))
        .route("/search", post(search::search::<RegularAppState>))
        .with_state(state);
}
