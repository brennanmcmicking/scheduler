use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::services::ServeDir;

use crate::middlewares;

mod calendar;
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
    let state = RegularAppState { courses };

    let calendar_route = Router::new()
        .route("/", put(calendar::add_to_calendar::<RegularAppState>))
        .route("/", delete(calendar::rm_from_calendar::<RegularAppState>));

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root::<RegularAppState>))
        .route("/search", post(search::search::<RegularAppState>))
        .nest("/calendar", calendar_route)
        .with_state(state)
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(axum::middleware::from_fn(middlewares::parse_cookie)),
        )
}
