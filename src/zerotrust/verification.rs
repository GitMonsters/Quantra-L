use anyhow::Result;
use std::collections::HashMap;
use chrono::{Utc, Duration};
use crate::zerotrust::SecureConnection;

/// Continuous Verifier monitors active connections
pub struct ContinuousVerifier {
    connections: HashMap<String, SecureConnection>,
    verification_interval: Duration,
}

impl ContinuousVerifier {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            verification_interval: Duration::minutes(5),
        }
    }

    pub async fn register_connection(&mut self, connection: SecureConnection) -> Result<()> {
        self.connections.insert(connection.id.clone(), connection);
        Ok(())
    }

    pub async fn unregister_connection(&mut self, connection_id: &str) -> Result<()> {
        self.connections.remove(connection_id);
        Ok(())
    }

    pub async fn verify(&self, connection_id: &str) -> Result<bool> {
        if let Some(conn) = self.connections.get(connection_id) {
            let time_since_last_verify = Utc::now() - conn.last_verified;
            Ok(time_since_last_verify < self.verification_interval)
        } else {
            Ok(false)
        }
    }

    pub async fn get_connection(&self, connection_id: &str) -> Result<Option<SecureConnection>> {
        Ok(self.connections.get(connection_id).cloned())
    }

    pub async fn get_all_connections(&self) -> Result<Vec<SecureConnection>> {
        Ok(self.connections.values().cloned().collect())
    }
}
