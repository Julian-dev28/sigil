//! Binary entry point for the Sigil HTTP service.

use tokio::net::TcpListener;

use sigil_server::{build_router, build_state, init_tracing, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::from_env();
    let state = build_state(&config).await?;
    let app = build_router(state.clone());

    let listener = TcpListener::bind(&config.bind).await?;
    tracing::info!(
        bind = %config.bind,
        issuer_did = %state.issuer_did,
        base_url = %state.base_url,
        "sigil-server listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Resolve when the process receives Ctrl-C or (on Unix) SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received; draining connections");
}
