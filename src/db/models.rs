use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct UserProfile {
    pub public_key: Vec<u8>,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub custom_avatar: Option<Vec<u8>>,
    pub encryption_pubkey: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserProfile {
    pub async fn find_by_pubkey(
        pool: &sqlx::PgPool,
        public_key: &[u8],
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, UserProfile>("SELECT * FROM users WHERE public_key = $1")
            .bind(public_key)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &sqlx::PgPool,
        public_key: &[u8],
        encryption_pubkey: &[u8],
        first_name: &str,
        username: Option<&str>,
        last_name: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, UserProfile>(
            "INSERT INTO users (public_key, encryption_pubkey, first_name, username, last_name)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(public_key)
        .bind(encryption_pubkey)
        .bind(first_name)
        .bind(username)
        .bind(last_name)
        .fetch_one(pool)
        .await
    }

    pub async fn update_profile(
        pool: &sqlx::PgPool,
        public_key: &[u8],
        encryption_pubkey: &[u8],
        first_name: &str,
        username: Option<&str>,
        last_name: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, UserProfile>(
            "UPDATE users
             SET encryption_pubkey = $2, first_name = $3, username = $4, last_name = $5
             WHERE public_key = $1
             RETURNING *",
        )
        .bind(public_key)
        .bind(encryption_pubkey)
        .bind(first_name)
        .bind(username)
        .bind(last_name)
        .fetch_one(pool)
        .await
    }
}
