//! steerd: runs the steering demos and streams their frames to the web app.
//!
//! Usage: steerd. Listens on http://127.0.0.1:7400; `/ws` is the client WebSocket.

mod contract;
mod ws;

use std::net::SocketAddr;

use anyhow::Result;
use axum::routing::get;
use axum::Router;
use tracing_subscriber::EnvFilter;

const PORT: u16 = 7400;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let app = Router::new().route("/ws", get(ws::handler));
    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(event = "host.listen", %addr);
    axum::serve(listener, app).await?;
    Ok(())
}
