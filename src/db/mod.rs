pub mod models;
mod pending;

pub use pending::PendingMessage;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(150)
        .connect(database_url)
        .await
}
