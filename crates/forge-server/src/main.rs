//! `forge-server` binary: read config from the environment and serve.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let level = std::env::var("FORGE_LOG_LEVEL").unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&level)),
        )
        .init();

    let config = match forge_server::Config::from_env() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };
    forge_server::run(config).await
}
