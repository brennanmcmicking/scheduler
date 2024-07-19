use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

mod root;
mod search;

pub fn make_app() -> Router {
    return Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root()))
        .route("/search", post(search::search()));
}
