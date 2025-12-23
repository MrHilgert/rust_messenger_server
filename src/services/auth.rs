use crate::db::models::UserProfile;
use crate::session::SessionManager;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AuthService {
    session_manager: Arc<SessionManager>,
    db_pool: PgPool,
    challenges: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl AuthService {
    pub fn new(session_manager: Arc<SessionManager>, db_pool: PgPool) -> Self {
        Self {
            session_manager,
            db_pool,
            challenges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn generate_challenge(&self, public_key: Vec<u8>) -> Vec<u8> {
        use rand::RngCore;

        let mut challenge = vec![0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut challenge);

        self.challenges
            .lock()
            .await
            .insert(public_key, challenge.clone());

        challenge
    }

    pub async fn verify_login(
        &self,
        public_key: &[u8],
        signature: &[u8],
    ) -> Result<(bool, bool), Box<dyn std::error::Error>> {
        let challenge = self.challenges.lock().await.remove(public_key);

        let challenge = match challenge {
            Some(c) => c,
            None => return Ok((false, false)),
        };

        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let verifying_key = VerifyingKey::from_bytes(
            public_key
                .try_into()
                .map_err(|_| "Invalid public key length")?,
        )?;

        let signature = Signature::from_bytes(
            signature
                .try_into()
                .map_err(|_| "Invalid signature length")?,
        );

        match verifying_key.verify(&challenge, &signature) {
            Ok(_) => (),
            Err(e) => return Ok((false, false)),
        }

        let profile = UserProfile::find_by_pubkey(&self.db_pool, public_key).await?;
        let profile_exists = profile.is_some();

        self.session_manager.set_authenticated(public_key).await;

        Ok((true, profile_exists))
    }
}
