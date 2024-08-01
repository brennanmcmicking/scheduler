use scheduler::routes;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                [
                    "backend=debug",         // code in this file
                    "scheduler=debug",       // code in this crate (but not this file)
                    "tower_http=debug",      // http request/response pairs
                    "axum::rejection=trace", // extractor rejections (i.e. bad form input)
                ]
                .join(",")
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // build our application with a route
    let app = routes::make_app().await;
    // run our app with hyper, listening globally on port
    let soc: std::net::SocketAddr = "0.0.0.0:8080"
        .parse()
        .expect("invalid binding socket address");
    let listener = tokio::net::TcpListener::bind(&soc).await.unwrap();
    info!("listening on http://{}", &soc);

    axum::serve(listener, app).await.unwrap();
}
