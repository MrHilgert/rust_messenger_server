use crate::session::Session;
use hnet_protocol::Packet;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use lrumap::LruHashMap;
use tokio::sync::Mutex;
use dashmap::DashMap;

pub struct SessionManager {
    sessions: Arc<DashMap<Vec<u8>, Session>>,
    session_enc_pubkeys: Arc<Mutex<LruHashMap<Vec<u8>, Vec<u8>>>>
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            session_enc_pubkeys: Arc::new(Mutex::new(LruHashMap::new(10_000)))
        }
    }

    pub fn add_session(&self, public_key: Vec<u8>, session: Session) {
        self.sessions.insert(public_key, session);
    }

    pub fn remove_session(&self, public_key: &[u8]) {
        self.sessions.remove(public_key);
    }

    pub async fn send_to_user(
        &self,
        public_key: &[u8],
        packet: Packet,
    ) -> Result<(), std::io::Error> {
        if let Some(mut session) = self.sessions.get_mut(public_key) {
            let requires_auth = !matches!(
                packet,
                Packet::Challenge { .. }
                    | Packet::LoginResponse { .. }
                    | Packet::MessageDelivered { .. }
                    | Packet::ProfileUpdated { .. }
                    | Packet::MessageReceived { .. }
                    | Packet::Ping
                    | Packet::Pong
                    | Packet::SearchUser { .. }
                    | Packet::UserFound { .. }
                    | Packet::UserNotFound
            );

            if requires_auth && !session.authenticated {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "Session not authenticated",
                ));
            }

            let raw = packet.to_raw();
            raw.write_to(&mut session.write_half).await?;
            session.update_activity();
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "User not found",
            ))
        }
    }

    pub async fn _send_to_users(
        &self,
        public_keys: &[Vec<u8>],
        packet: Packet,
    ) -> Result<(), std::io::Error> {
        let raw = packet.to_raw();

        for pubkey in public_keys {
            if let Some(mut session) = self.sessions.get_mut(pubkey) {
                raw.write_to(&mut session.write_half).await?;
                session.update_activity();
            }
        }

        Ok(())
    }

    pub async fn _broadcast(&self, packet: Packet) -> Result<(), std::io::Error> {
        let raw = packet.to_raw();

        for mut session in self.sessions.iter_mut() {
            raw.write_to(&mut session.write_half).await?;
            session.update_activity();
        }

        Ok(())
    }

    pub async fn set_authenticated(&self, public_key: &[u8]) {
        if let Some(mut session) = self.sessions.get_mut(public_key) {
            session.authenticated = true;
        }
    }

    pub async fn move_session(&self, old_key: &[u8], new_key: Vec<u8>) {
        if let Some((_, mut session)) = self.sessions.remove(old_key) {
            session.public_key = new_key.clone();
            self.sessions.insert(new_key, session);
        }
    }

    pub async fn put_session_enc_pubkey(&self, auth_pub_key: Vec<u8>, enc_pub_key: Vec<u8>) {
        let mut enc_pubkeys = self.session_enc_pubkeys.lock().await;

        enc_pubkeys.push(auth_pub_key, enc_pub_key);
    }

    pub async fn get_session_enc_pubkey(&self, auth_pub_key: Vec<u8>) -> Option<Vec<u8>> {
        let mut enc_pubkeys = self.session_enc_pubkeys.lock().await;

        enc_pubkeys.get(&auth_pub_key).cloned()
    }
}
