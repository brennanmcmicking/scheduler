use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use scheduler::{routes::{self, Stage}, scraper::scrape};
use tokio::{task, time};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stage: Stage = env::var("STAGE").map_or(Stage::LOCAL, |v| 
        v.into()
    );
    let use_local_dynamo = env::var("USE_LOCAL_DYNAMO").is_ok_and(|v| 
       v == "true"
    );

    match stage {
        Stage::PROD => {
            tracing_subscriber::registry()
                .with(
                tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                    [
                        "backend=info",         // code in this file
                        "scheduler=info",       // code in this crate (but not this file)
                        "tower_http=info",      // http request/response pairs
                        "axum::rejection=trace", // extractor rejections (i.e. bad form input)
                    ]
                        .join(",")
                        .into()
                    }),
                )
                .with(tracing_subscriber::fmt::layer())
                .init();

            let _forever = task::spawn(async {
                let mut interval = time::interval(Duration::from_secs(3600)); // once per hour
                loop {
                    interval.tick().await;
                    debug!("running scraper");
                    let _r = scrape(true, None).await;
                }
            });
        },
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

    // build our application with a route
    let app = routes::make_app(stage, use_local_dynamo).await;
    // run our app with hyper, listening globally on port
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("/scheduler/fullchain.pem"), 
        PathBuf::from("/scheduler/privkey.pem"),
    )
    .await;

    let soc: SocketAddr = "0.0.0.0:8443"
    .parse()
    .context("invalid binding socket address")?;

    // if the TLS certs are present, bind with HTTPS
    // otherwise, run normally
    match config {
        Ok(c) => { 
            axum_server::bind_rustls(soc, c)
                .serve(app.into_make_service())
                .await
                .context("error while serving HTTPS app")?;
        },
        Err(_e) => {
            axum_server::bind(soc)
                .serve(app.into_make_service())
                .await
                .context("error while serving HTTP app")?;
        }
    }

    Ok(())
}