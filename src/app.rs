use std::{
    env::{self, current_dir},
    sync::Arc,
};

use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_dynamodb::types::TimeToLiveSpecification;
use axum::{
    extract::Request,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Router,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use reqwest::StatusCode;
use tower_http::{
    services::ServeDir,
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{debug, debug_span};

use crate::{
    common::{AppError, Stage},
    data::{store::DynamoUserStore, DatabaseAppState},
    routes::{calendar, donate, generate, import, login, preview, root, schedule, search, share},
};

pub async fn make_app(stage: Stage, use_local_dynamo: bool) -> Router {
    let region = RegionProviderChain::default_provider().or_else("us-east-1");
    let ddb_config = match use_local_dynamo {
        false => {
            aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .load()
                .await
        }
        true => {
            aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .endpoint_url("http://localhost:8000")
                .test_credentials()
                .load()
                .await
        }
    };
    let schedules_table_name = match stage {
        Stage::PROD => "schedules".to_string(),
        Stage::LOCAL => "schedules-dev".to_string(),
    };
    let sessions_table_name = match stage {
        Stage::PROD => "sessions".to_string(),
        Stage::LOCAL => "sessions-dev".to_string(),
    };

    let ddb_client = aws_sdk_dynamodb::Client::new(&ddb_config);

    let table_list = ddb_client.list_tables().send().await.unwrap();
    if !table_list.table_names().contains(&schedules_table_name) {
        let _ = DatabaseAppState::create_table(
            &ddb_client,
            &schedules_table_name,
            "userId",
            "scheduleId",
        )
        .await
        .map_err(|_e| panic!());
    }
    if !table_list.table_names().contains(&sessions_table_name) {
        let _ = DatabaseAppState::create_table(
            &ddb_client,
            &sessions_table_name,
            "userId",
            "sessionId",
        )
        .await
        .map_err(|_e| panic!());
    }
    // will set TTL on the table even if it already existed for backwards-compat reasons
    let _ = ddb_client
        .update_time_to_live()
        .table_name(&sessions_table_name)
        .time_to_live_specification(
            TimeToLiveSpecification::builder()
                .attribute_name("expiresAt")
                .enabled(true)
                .build()
                .unwrap(),
        )
        .send()
        .await;

    let user_store = DynamoUserStore::new(ddb_client, &sessions_table_name, &schedules_table_name);

    type State = Arc<DatabaseAppState>;

    let discord_secret = env::var("DISCORD_SECRET").unwrap_or("".to_string());

    let state: State = Arc::new(
        DatabaseAppState::new(
            current_dir().expect("couldn't access current directory"),
            stage,
            user_store,
            &discord_secret,
        )
        .await
        .expect("failed to initialize database state"),
    );

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root))
        .nest(
            "/login",
            Router::new()
                .route("/", get(login::get))
                .route("/google", post(login::post_google))
                .route("/discord", get(login::get_discord)),
        )
        .route("/share/:schedule_id", get(share::get))
        .route("/import", get(import::get))
        .route("/donate", get(donate::get))
        .route("/schedule", post(schedule::post))
        .nest(
            "/schedule/:schedule_id",
            Router::new()
                .route("/", get(schedule::get))
                .route("/", delete(schedule::delete))
                .route("/search", post(search::search))
                .route("/generate", get(generate::get).post(generate::post))
                .nest(
                    "/calendar",
                    Router::new()
                        .route("/", get(calendar::get_calendar))
                        .route("/", put(calendar::add_to_calendar))
                        .route("/", patch(calendar::update_calendar))
                        .route("/", delete(calendar::rm_from_calendar))
                        .route("/preview", get(preview::preview)),
                )
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    schedule::not_found,
                )),
        )
        .with_state(state)
        .layer(middleware::from_fn(unauth_redirect))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    debug_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_response(DefaultOnResponse::new().latency_unit(LatencyUnit::Micros)),
        )
}

async fn unauth_redirect(req: Request, next: Next) -> Result<impl IntoResponse, AppError> {
    let res = next.run(req).await;
    debug!("res.status()={}", res.status());
    if res.status() == StatusCode::UNAUTHORIZED {
        // if there was an unauthorized response then delete the session cookie and redirect to the login page
        let cookie = Cookie::build(("session", "")).removal().build();
        return Ok((
            CookieJar::new().add(cookie),
            [("location", "/login")],
            StatusCode::SEE_OTHER,
        )
            .into_response());
    }

    Ok(res)
}
