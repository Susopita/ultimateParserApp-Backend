use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

mod core;
mod api;
mod parsers;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "Ultimate Parser API - Phase 3 Active" }))
        .route("/analyze-grammar", post(api::analyze_grammar))
        .route("/parse-rd", post(api::parse_rd))
        .route("/parse-ll1", post(api::parse_ll1))
        .route("/parse-lr0", post(api::parse_lr0))
        .route("/parse-slr1", post(api::parse_slr1))
        .route("/parse-lr1", post(api::parse_lr1))
        .route("/parse-lalr1", post(api::parse_lalr1))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("🚀 Backend server running on http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
