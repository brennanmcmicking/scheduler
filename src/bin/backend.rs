use scheduler::routes;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let mut courses: Vec<String> = Vec::new();
    courses.push("math100".to_string());
    courses.push("csc111".to_string());
    courses.push("engr110".to_string());
    courses.push("math122".to_string());
    courses.push("math110".to_string());
    courses.push("engr141".to_string());
    courses.push("csc225".to_string());
    courses.push("phys111".to_string());
    courses.push("ece260".to_string());
    courses.push("seng265".to_string());
    courses.push("seng475".to_string());
    courses.push("seng371".to_string());
    courses.push("math109".to_string());
    courses.push("csc115".to_string());
    courses.push("csc320".to_string());
    courses.push("phys120".to_string());

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
