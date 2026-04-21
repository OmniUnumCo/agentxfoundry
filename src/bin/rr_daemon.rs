//! rr-daemon — Rational Reserve daemon binary entry point.
//!
//! Usage:
//!   rr-daemon              # Run as background daemon (via ensure_daemon from shim)
//!   rr-daemon --foreground  # Run in foreground (for tests/debugging)

use rr::daemon::Daemon;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    let foreground = args.iter().any(|a| a == "--foreground");

    if foreground {
        eprintln!("Starting rr-daemon in foreground mode");
    }

    let mut daemon = Daemon::new()?;

    if foreground {
        daemon.serve_forever().await?;
    } else {
        // In production the daemon is launched via ensure_daemon() from the shim.
        // When invoked directly without --foreground, just run in foreground anyway
        // since we have no fork/detach mechanism in the binary itself.
        daemon.serve_forever().await?;
    }

    Ok(())
}
