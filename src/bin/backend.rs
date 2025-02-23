use std::{env, net::SocketAddr, path::PathBuf, str::FromStr, time::Duration};

use anyhow::Context;
use axum_server::{tls_rustls::RustlsConfig, Handle};
use scheduler::{app, common::Stage, scraper::{self, Term}};
use tokio::{task, time};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stage: Stage = env::var("STAGE").map_or(Stage::LOCAL, |v| v.into());
    let use_local_dynamo = env::var("USE_LOCAL_DYNAMO").is_ok_and(|v| v == "true");

    match stage {
        Stage::PROD => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                        [
                            "backend=info",          // code in this file
                            "scheduler=info",        // code in this crate (but not this file)
                            "tower_http=info",       // http request/response pairs
                            "axum::rejection=trace", // extractor rejections (i.e. bad form input)
                        ]
                        .join(",")
                        .into()
                    }),
                )
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
        _ => tracing_subscriber::registry()
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
            .init(),
    };
    // initialize tracing

    // run our app with hyper, listening globally on port
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("/scheduler/fullchain.pem"),
        PathBuf::from("/scheduler/privkey.pem"),
    )
    .await;

    let soc: SocketAddr = "0.0.0.0:8443"
        .parse()
        .context("invalid binding socket address")?;

    debug!("starting server");
    
    // if the TLS certs are present, bind with HTTPS
    // otherwise, run normally
    match config {
        Ok(c) => {
            let mut interval = time::interval(Duration::from_secs(60 * 60)); // every hour
            let mut app = app::make_app(stage.clone(), use_local_dynamo).await;
            loop {
                let handle = Handle::new();
                let handle_clone = handle.clone();
                let config_clone = c.clone();
                task::spawn(async move {
                    axum_server::bind_rustls(soc, config_clone)
                        .handle(handle_clone)
                        .serve(app.into_make_service())
                        .await
                        .context("error while serving HTTPS app").unwrap();
                });
                interval.tick().await;
                let _r = scraper::scrape(true, Some(Term::from_str("202409").unwrap())).await;
                app = app::make_app(stage.clone(), use_local_dynamo).await;
                handle.shutdown();
            }
        }
        Err(_e) => {
            let mut interval = time::interval(Duration::from_secs(60 * 10)); // every 10 minutes
            let mut app = app::make_app(stage.clone(), use_local_dynamo).await;
            loop {
                let handle = Handle::new();
                let handle_clone = handle.clone();
                debug!("shut down old app, binding new app");
                task::spawn(async move {
                    axum_server::bind(soc)
                        .handle(handle_clone)
                        .serve(app.into_make_service())
                        .await
                        .context("error while serving HTTP app").unwrap();
                });
                interval.tick().await;
                debug!("running scraper");
                let _r = scraper::scrape(true, Some(Term::from_str("202409").unwrap())).await;
                debug!("done scraping");
                app = app::make_app(stage.clone(), use_local_dynamo).await;
                handle.shutdown();
            }
        }
    }
}
