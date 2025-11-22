use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};

pub const QUANTRA_PROTOCOL: StreamProtocol = StreamProtocol::new("/quantra/1.0.0");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantraRequest {
    Ping,
    GetPeers,
    SendMessage { encrypted_data: Vec<u8> },
    GetQuote { symbol: String },
    ProvisionESim { profile_data: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantraResponse {
    Pong,
    Peers(Vec<String>),
    MessageSent,
    Quote { symbol: String, price: f64, timestamp: i64 },
    ESimProvisioned { activation_code: String },
    Error(String),
}
