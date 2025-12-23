use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PendingMessage {
    pub _id: i64,
    pub _recipient_pubkey: Vec<u8>,
    pub sender_pubkey: Vec<u8>,
    pub sender_enc_pubkey: Vec<u8>,
    pub encrypted_content: Vec<u8>,
    pub _created_at: DateTime<Utc>,
}

impl PendingMessage {
    pub async fn save(
        pool: &PgPool,
        recipient_pubkey: &[u8],
        sender_pubkey: &[u8],
        sender_enc_pubkey: &[u8],
        encrypted_content: Vec<u8>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO pending_messages (recipient_pubkey, sender_pubkey, sender_enc_pubkey, encrypted_content)
             VALUES ($1, $2, $3, $4)"
        )
            .bind(recipient_pubkey)
            .bind(sender_pubkey)
            .bind(sender_enc_pubkey)
            .bind(encrypted_content)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_and_delete_for_user(
        pool: &PgPool,
        recipient_pubkey: &[u8],
    ) -> Result<Vec<PendingMessage>, sqlx::Error> {
        let messages: Vec<_> = sqlx::query_as::<_, (i64, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, DateTime<Utc>)>(
            "SELECT id, recipient_pubkey, sender_pubkey, sender_enc_pubkey, encrypted_content, created_at
             FROM pending_messages
             WHERE recipient_pubkey = $1
             ORDER BY created_at ASC"
        )
            .bind(recipient_pubkey)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|(id, recipient_pubkey, sender_pubkey, sender_enc_pubkey, encrypted_content, created_at)| {
                PendingMessage {
                    _id: id,
                    _recipient_pubkey: recipient_pubkey,
                    sender_pubkey,
                    sender_enc_pubkey,
                    encrypted_content,
                    _created_at: created_at,
                }
            })
            .collect();

        if !messages.is_empty() {
            sqlx::query("DELETE FROM pending_messages WHERE recipient_pubkey = $1")
                .bind(recipient_pubkey)
                .execute(pool)
                .await?;
        }

        Ok(messages)
    }
}
