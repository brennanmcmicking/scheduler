use std::{net::SocketAddr, path::PathBuf};

use anyhow::{anyhow, Context};
use axum::{extract::Host, handler::HandlerWithoutStateExt, http::Uri, response::Redirect, BoxError};
use axum_server::tls_rustls::RustlsConfig;
use reqwest::StatusCode;
use scheduler::routes;
use tracing::{debug, info};
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
    let soc: SocketAddr = "0.0.0.0:8080"
        .parse()
        .context("invalid binding socket address")?;
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("/scheduler/fullchain.pem"), 
        PathBuf::from("/scheduler/privkey.pem"),
    )
    .await;


    // if the TLS certs are present, bind with HTTPS
    // otherwise, run normally
    match config {
        Ok(c) => { 
            info!("found TLS configuration, listening on 80 and 8080");
            tokio::spawn(redirect_http_to_https());
            axum_server::bind_rustls(soc, c)
                .serve(app.into_make_service())
                .await
                .unwrap();
        },
        Err(_e) => {
            info!("running non-https version on port 8080");
            let listener = tokio::net::TcpListener::bind(&soc)
                .await
                .with_context(|| anyhow!("failed to bind listener to {}", soc))?;
            axum::serve(listener, app)
                .await
                .context("error while serving app")?;
        }

    }

    Ok(())
}

// https://github.com/tokio-rs/axum/blob/9ec18b9a5d67f9dd453c45c07a42d2333c4acd08/examples/tls-rustls/src/main.rs
async fn redirect_http_to_https() {
    fn make_https(host: String, uri: Uri) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace("80", "443");
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 80));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    debug!("https upgrader listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}
