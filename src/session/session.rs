use std::time::Instant;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;

pub struct Session {
    pub public_key: Vec<u8>,
    pub write_half: WriteHalf<TcpStream>,
    pub authenticated: bool,
    pub last_activity: Instant,
}

impl Session {
    pub fn new(public_key: Vec<u8>, write_half: WriteHalf<TcpStream>) -> Self {
        Self {
            public_key,
            write_half,
            authenticated: false,
            last_activity: Instant::now(),
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn _is_authenticated(&self) -> bool {
        self.authenticated
    }
}
