use axum::{routing::get, Router};

mod root;

pub fn make_app() -> Router {
    return Router::new()
        // `GET /` goes to `root`
        .route("/", get(root::root()));
}
