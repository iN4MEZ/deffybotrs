use std::{net::SocketAddr, str::FromStr};

use axum::{
    Router, body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response,
    routing::get,
};
use tokio::{net::TcpListener, time::Instant};

mod routes;

pub async fn http_init() -> Result<(), anyhow::Error> {
    tokio::spawn(async {
        if let Err(e) = start_http().await {
            tracing::error!("Failed to start HTTP server: {:?}", e);
        }
    });
    Ok(())
}

async fn start_http() -> Result<(), std::io::Error> {
    let app = Router::new().route("/", get(root))
    .nest("/patreon/webhook", routes::patreon_webhook::routes().await);

    let addr_str = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:10000".to_string());

    let addr = SocketAddr::from_str(&addr_str).unwrap();

    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn _log_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;
    let duration = start.elapsed();

    tracing::info!(
        "Connection: {}  Request: {} {} - Response: {} - Duration: {:?}",
        addr,
        method,
        uri,
        response.status(),
        duration
    );

    response
}