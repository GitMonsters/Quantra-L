use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub public_key: Vec<u8>,
    pub nickname: Option<String>,
    pub last_seen: i64,
}

impl Peer {
    pub fn new(id: String, public_key: Vec<u8>) -> Self {
        Self {
            id,
            public_key,
            nickname: None,
            last_seen: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_nickname(mut self, nickname: String) -> Self {
        self.nickname = Some(nickname);
        self
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = chrono::Utc::now().timestamp();
    }
}
