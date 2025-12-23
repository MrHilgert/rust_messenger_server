mod db;
mod handlers;
mod hnet;
mod logging;
mod services;
mod session;

use hnet::server::Server;
use logging::Logger;
use tokio::signal;
use tokio_util::sync::CancellationToken;

async fn init(logger: Logger) -> Result<(), Box<dyn std::error::Error>> {
    let database_url =
        logger.log_err(std::env::var("DATABASE_URL"), "Database url not set in env")?;

    logger.d("Create DB pool");

    let db_pool = logger.log_err(
        db::create_pool(&database_url).await,
        "Error creating database pool",
    )?;

    logger.d("DB Pool created");

    let server = Server::new("127.0.0.1".to_string(), 8123, db_pool);
    let shutdown_token = CancellationToken::new();

    tokio::select! {
        _ = server.listen(shutdown_token.clone()) => {
            logger.i("Server stopped");
        }
        _ = shutdown_signal() => {
            logger.i("Received shutdown signal, stopping server...");
            shutdown_token.cancel();
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
    }

    #[cfg(windows)]
    {
        signal::ctrl_c().await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let logger = Logger::new("MAIN");

    #[cfg(debug_assertions)]
    {
        logger.w("Server started on debug version");
    }

    logger.i("Starting server");

    if let Err(_) = init(logger.clone()).await {
        std::process::exit(1);
    }
}
