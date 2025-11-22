use anyhow::Result;
use libp2p::PeerId;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct NetworkManager {
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<String>,
    pub last_seen: i64,
    pub reputation: i32,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_peer(&self, peer_id: PeerId, info: PeerInfo) {
        let mut peers = self.peers.write().await;
        peers.insert(peer_id, info);
    }

    pub async fn remove_peer(&self, peer_id: &PeerId) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
    }

    pub async fn get_peer(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }

    pub async fn get_all_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    pub async fn update_peer_reputation(&self, peer_id: &PeerId, delta: i32) -> Result<()> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.reputation += delta;
        }
        Ok(())
    }
}
