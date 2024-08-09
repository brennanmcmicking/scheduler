use std::net::Ipv4Addr;

use anyhow::{anyhow, Context};
use scheduler::routes;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        .context("invalid binding socket address")?;
    let listener = tokio::net::TcpListener::bind(&soc)
        .await
        .with_context(|| anyhow!("failed to bind listener to {}", soc))?;
    info!("listening on http://{}", &soc);
    if soc.ip() == std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)) {
        info!(
            "for local dev, please use  http://127.0.0.1:{} to make secure cookies work",
            &soc.port()
        );
    }

    axum::serve(listener, app)
        .await
        .context("error while serving app")?;

    Ok(())
}
