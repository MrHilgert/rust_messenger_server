use crate::logging::Logger;
use crate::services::{AuthService, MessageService, UserService};
use crate::session::SessionManager;
use hnet_protocol::Packet;
use std::sync::Arc;

pub struct PacketHandler {
    auth_service: Arc<AuthService>,
    user_service: Arc<UserService>,
    message_service: Arc<MessageService>,
    session_manager: Arc<SessionManager>,
    logger: Logger,
}

impl PacketHandler {
    pub fn new(
        auth_service: Arc<AuthService>,
        user_service: Arc<UserService>,
        message_service: Arc<MessageService>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self {
            auth_service,
            user_service,
            message_service,
            session_manager,
            logger: Logger::new("NETWORK"),
        }
    }

    pub async fn handle(
        &self,
        sender_pubkey: Option<Vec<u8>>,
        packet: Packet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match packet {
            Packet::GetChallenge { public_key } => {
                let challenge = self
                    .auth_service
                    .generate_challenge(public_key.clone())
                    .await;

                if let Some(sender) = sender_pubkey {
                    self.session_manager
                        .send_to_user(&sender, Packet::Challenge { challenge })
                        .await?;
                }
            }

            Packet::LoginRequest {
                public_key,
                signature,
            } => {
                let (success, profile_exists) = self
                    .auth_service
                    .verify_login(&public_key, &signature)
                    .await?;

                if let Some(sender) = sender_pubkey {
                    self.session_manager
                        .send_to_user(
                            &sender,
                            Packet::LoginResponse {
                                success,
                                profile_exists,
                            },
                        )
                        .await?;
                }
            }

            Packet::SetProfile {
                encryption_pubkey,
                first_name,
                username,
                last_name,
            } => {
                if let Some(pubkey) = sender_pubkey {
                    self.user_service
                        .set_profile(&pubkey, encryption_pubkey, first_name, username, last_name)
                        .await?;
                }
            }

            Packet::SearchUser { query } => {
                if let Some(pubkey) = sender_pubkey {
                    self.user_service.search_user(&pubkey, query).await?;
                }
            }

            Packet::SendMessage {
                recipient_pubkey,
                encrypted_content,
            } => {
                if let Some(sender) = sender_pubkey {
                    if let Ok(sender_enc_pubkey) =
                        self.user_service.get_encryption_pubkey(&sender).await
                    {
                        self.message_service
                            .route_message(
                                &sender,
                                &sender_enc_pubkey,
                                recipient_pubkey,
                                encrypted_content,
                            )
                            .await?;
                    }
                }
            }

            _ => {
                self.logger
                    .w(&format!("Unhandled packet id: {:02X}", packet.get_id()));
            }
        }

        Ok(())
    }
}
