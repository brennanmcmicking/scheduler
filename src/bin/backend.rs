use scheduler::routes;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let courses: Vec<String> = vec![
        "math100".to_string(),
        "csc111".to_string(),
        "engr110".to_string(),
        "math122".to_string(),
        "math110".to_string(),
        "engr141".to_string(),
        "csc225".to_string(),
        "phys111".to_string(),
        "ece260".to_string(),
        "seng265".to_string(),
        "seng475".to_string(),
        "seng371".to_string(),
        "math109".to_string(),
        "csc115".to_string(),
        "csc320".to_string(),
        "phys120".to_string(),
    ];

    // build our application with a route
    let app = routes::make_app(courses).await;

    // run our app with hyper, listening globally on port
    let soc: std::net::SocketAddr = "0.0.0.0:8080"
        .parse()
        .expect("invalid binding socket address");
    println!("binding socket to {}", &soc);
    let listener = tokio::net::TcpListener::bind(&soc).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
