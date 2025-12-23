use colored::Colorize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use tokio_util::sync::CancellationToken;

use crate::handlers::PacketHandler;
use crate::logging::Logger;
use crate::services::{AuthService, MessageService, UserService};
use crate::session::{Session, SessionManager};
use hnet_protocol::{Packet, RawPacket};

pub struct Server {
    host: String,
    port: u16,
    logger: Logger,
    session_manager: Arc<SessionManager>,
    packet_handler: Arc<PacketHandler>,
    message_service: Arc<MessageService>,
}

impl Server {
    pub fn new(host: String, port: u16, db_pool: PgPool) -> Self {
        let session_manager = Arc::new(SessionManager::new());

        let auth_service = Arc::new(AuthService::new(
            Arc::clone(&session_manager),
            db_pool.clone(),
        ));

        let user_service = Arc::new(UserService::new(
            Arc::clone(&session_manager),
            db_pool.clone(),
        ));

        let message_service = Arc::new(MessageService::new(
            Arc::clone(&session_manager),
            db_pool.clone(),
        ));

        let packet_handler = Arc::new(PacketHandler::new(
            auth_service,
            user_service,
            Arc::clone(&message_service),
            Arc::clone(&session_manager),
        ));

        Self {
            host,
            logger: Logger::new("SERVER"),
            port,
            session_manager,
            packet_handler,
            message_service,
        }
    }

    pub async fn listen(&self, shutdown_token: CancellationToken) {
        let addr = format!("{}:{}", self.host, self.port);

        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                self.logger.e(&format!("Failed to bind to {}: {}", addr, e));
                return;
            }
        };

        self.logger
            .i(&format!("Server listening on {}", addr.bright_green()));

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            self.logger.d(&format!("New connection from {}", addr.to_string().bright_magenta()));

                            let session_manager = Arc::clone(&self.session_manager);
                            let packet_handler = Arc::clone(&self.packet_handler);
                            let message_service = Arc::clone(&self.message_service);
                            let logger = self.logger.clone();
                            let shutdown_token = shutdown_token.child_token();

                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(
                                    stream,
                                    session_manager,
                                    packet_handler,
                                    message_service,
                                    shutdown_token,
                                    logger.clone(),
                                ).await {
                                    logger.e(&format!("Connection error: {}", e));
                                }
                            });
                        }
                        Err(e) => self.logger.e(&format!("Failed to accept connection: {}", e)),
                    }
                }

                _ = shutdown_token.cancelled() => {
                    self.logger.i("Shutdown signal received, stopping listener");
                    break;
                }
            }
        }

        self.logger.i("Server stopped accepting new connections");
    }
}

async fn handle_connection(
    stream: TcpStream,
    session_manager: Arc<SessionManager>,
    packet_handler: Arc<PacketHandler>,
    message_service: Arc<MessageService>,
    shutdown_token: CancellationToken,
    logger: Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr()?;
    let (mut read_half, write_half) = tokio::io::split(stream);

    let temp_id = format!("{}", peer_addr).into_bytes();

    session_manager
        .add_session(temp_id.clone(), Session::new(temp_id.clone(), write_half));

    let mut current_user: Option<Vec<u8>> = Some(temp_id.clone());
    let timeout_duration = tokio::time::Duration::from_secs(90);

    loop {
        tokio::select! {
            result = tokio::time::timeout(timeout_duration, RawPacket::read_from(&mut read_half)) => {
                match result {
                    Ok(Ok(raw)) => {
                        match Packet::from_raw(raw) {
                            Ok(Packet::Ping) => {
                                if let Some(ref user) = current_user {
                                    let _ = session_manager.send_to_user(user, Packet::Pong).await;
                                }
                            }

                            Ok(Packet::LoginRequest { ref public_key, signature }) => {
                                let pk = public_key.clone();

                                if let Err(e) = packet_handler.handle(
                                    current_user.clone(),
                                    Packet::LoginRequest {
                                        public_key: public_key.clone(),
                                        signature,
                                    }
                                ).await {
                                    logger.e(&format!("Failed to handle login request: {}", e));
                                }

                                session_manager.move_session(&temp_id, pk.clone()).await;
                                current_user = Some(pk.clone());

                                tokio::spawn({
                                    let message_service = Arc::clone(&message_service);
                                    let pubkey = pk.clone();
                                    let logger = logger.clone();
                                    async move {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                        if let Err(e) = message_service.deliver_pending_messages(&pubkey).await {
                                            logger.e(&format!("Failed to deliver pending messages: {}", e));
                                        }
                                    }
                                });

                                continue;
                            }

                            Ok(packet) => {
                                if let Err(e) = packet_handler.handle(current_user.clone(), packet).await {
                                    logger.e(&format!("Failed to handle packet: {}", e));
                                }
                            }

                            Err(e) => logger.w(&format!("Received invalid packet: {:?}", e)),
                        }
                    }
                    Ok(Err(e)) => {
                        logger.d(&format!("Connection read error: {}", e));
                        break;
                    }
                    Err(_) => {
                        logger.d(&format!("Connection timeout after {} seconds of inactivity", timeout_duration.as_secs()));
                        break;
                    }
                }
            }

            _ = shutdown_token.cancelled() => {
                logger.i("Connection closing due to server shutdown");
                break;
            }
        }
    }

    if let Some(user_id) = current_user {
        session_manager.remove_session(&user_id);
        logger.d(&format!(
            "User disconnected: {}",
            hex::encode(&user_id[..4])
        ));
    }

    Ok(())
}
