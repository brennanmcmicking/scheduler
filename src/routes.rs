use axum::{routing::get, Router};
use tower_http::services::ServeDir;

mod root;

pub fn make_app() -> Router {
    return Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root()));
}
