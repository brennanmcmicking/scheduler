use scheduler::routes;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = routes::make_app().await;

    // run our app with hyper, listening globally on port
    let soc: std::net::SocketAddr = "0.0.0.0:8080"
        .parse()
        .expect("invalid binding socket address");
    println!("binding socket to {}", &soc);
    let listener = tokio::net::TcpListener::bind(&soc).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
