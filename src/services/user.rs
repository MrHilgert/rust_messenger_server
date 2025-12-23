use crate::db::models::UserProfile;
use crate::session::SessionManager;
use hnet_protocol::Packet;
use sqlx::PgPool;
use std::sync::Arc;

pub struct UserService {
    session_manager: Arc<SessionManager>,
    db_pool: PgPool,
}

impl UserService {
    pub fn new(session_manager: Arc<SessionManager>, db_pool: PgPool) -> Self {
        Self {
            session_manager,
            db_pool,
        }
    }

    pub async fn set_profile(
        &self,
        public_key: &[u8],
        encryption_pubkey: Vec<u8>,
        first_name: String,
        username: Option<String>,
        last_name: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let existing = UserProfile::find_by_pubkey(&self.db_pool, public_key).await?;

        if existing.is_some() {
            UserProfile::update_profile(
                &self.db_pool,
                public_key,
                &encryption_pubkey,
                &first_name,
                username.as_deref(),
                last_name.as_deref(),
            )
            .await?;
        } else {
            UserProfile::create(
                &self.db_pool,
                public_key,
                &encryption_pubkey,
                &first_name,
                username.as_deref(),
                last_name.as_deref(),
            )
            .await?;
        }

        self.session_manager
            .send_to_user(public_key, Packet::ProfileUpdated { success: true })
            .await?;

        Ok(())
    }

    pub async fn search_user(
        &self,
        requester_pubkey: &[u8],
        query: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, UserProfile>("SELECT * FROM users WHERE username = $1")
            .bind(&query)
            .fetch_optional(&self.db_pool)
            .await?;

        if let Some(user) = user {
            self.session_manager
                .send_to_user(
                    requester_pubkey,
                    Packet::UserFound {
                        public_key: user.public_key,
                        encryption_pubkey: user.encryption_pubkey,
                        username: user.username,
                        first_name: user.first_name,
                        last_name: user.last_name,
                    },
                )
                .await?;
        } else {
            self.session_manager
                .send_to_user(requester_pubkey, Packet::UserNotFound)
                .await?;
        }

        Ok(())
    }

    pub async fn get_encryption_pubkey(
        &self,
        auth_pubkey: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {

        if let Some(enc_pubkey) = self.session_manager.get_session_enc_pubkey(auth_pubkey.to_vec()).await {
            Ok(enc_pubkey)
        } else {
            if let Some(profile) = UserProfile::find_by_pubkey(&self.db_pool, auth_pubkey).await? {
                self.session_manager.put_session_enc_pubkey(auth_pubkey.to_vec(), profile.encryption_pubkey.clone()).await;
                Ok(profile.encryption_pubkey)
            } else {
                Err("User not found".into())
            }
        }
    }
}
