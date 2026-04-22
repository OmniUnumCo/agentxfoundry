//! `rr-http` — HTTP server binding the axum router from `api_routes` against
//! a live `DaemonState`.
//!
//! Intended as a sibling of `rr-daemon`: they serve the same state through
//! different transports. Run `rr-daemon` for the MCP unix-socket path and
//! `rr-http` for REST.

use rr::api_routes::build_router;
use rr::daemon::Daemon;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let daemon = Daemon::new()?;
    let state = Arc::new(daemon.state_clone());
    let app = build_router(state);

    let bind = std::env::var("RR_HTTP_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let addr: SocketAddr = bind.parse()?;
    let listener = TcpListener::bind(addr).await?;

    eprintln!("rr-http listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
