use crate::db::PendingMessage;
use crate::session::SessionManager;
use hnet_protocol::Packet;
use sqlx::PgPool;
use std::sync::Arc;

pub struct MessageService {
    session_manager: Arc<SessionManager>,
    db_pool: PgPool,
}

impl MessageService {
    pub fn new(session_manager: Arc<SessionManager>, db_pool: PgPool) -> Self {
        Self {
            session_manager,
            db_pool,
        }
    }

    pub async fn route_message(
        &self,
        sender_pubkey: &[u8],
        sender_enc_pubkey: &[u8],
        recipient_pubkey: Vec<u8>,
        encrypted_content: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let delivered = self
            .session_manager
            .send_to_user(
                &recipient_pubkey,
                Packet::MessageReceived {
                    sender_pubkey: sender_pubkey.to_vec(),
                    sender_enc_pubkey: sender_enc_pubkey.to_vec(),
                    encrypted_content: encrypted_content.clone(),
                },
            )
            .await
            .is_ok();

        if !delivered {
            PendingMessage::save(
                &self.db_pool,
                &recipient_pubkey,
                sender_pubkey,
                sender_enc_pubkey,
                encrypted_content,
            )
            .await?;
        }

        let result = self
            .session_manager
            .send_to_user(sender_pubkey, Packet::MessageDelivered { success: true })
            .await;

        if let Err(e) = result {
            return Err(e.into());
        }

        Ok(())
    }

    pub async fn deliver_pending_messages(
        &self,
        user_pubkey: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pending = PendingMessage::get_and_delete_for_user(&self.db_pool, user_pubkey).await?;

        if !pending.is_empty() {
            for msg in pending {
                self.session_manager
                    .send_to_user(
                        user_pubkey,
                        Packet::MessageReceived {
                            sender_pubkey: msg.sender_pubkey,
                            sender_enc_pubkey: msg.sender_enc_pubkey,
                            encrypted_content: msg.encrypted_content,
                        },
                    )
                    .await?;
            }
        }

        Ok(())
    }
}
