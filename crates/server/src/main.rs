use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustune_server=info".into()),
        )
        .init();

    let port = std::env::var("RUSTUNE_PORT")
        .unwrap_or_else(|_| "3867".into())
        .parse::<u16>()?;
    if port == 0 {
        return Err("RUSTUNE_PORT must be between 1 and 65535".into());
    }
    let web_dist = std::env::var_os("RUSTUNE_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/web/dist"));
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "Rustune foundation server started");
    axum::serve(listener, rustune_server::router(&web_dist))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("register SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
