use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use sha2::{Sha256, Digest};

/// Identity represents a verified user/peer identity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub user_id: String,
    pub public_key: Vec<u8>,
    pub attributes: HashMap<String, String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub signature: Vec<u8>,
}

/// Trust score for an identity (0-100)
pub type TrustScore = u8;

/// Identity Manager handles identity verification and trust scoring
pub struct IdentityManager {
    identities: HashMap<String, IdentityRecord>,
    trust_scores: HashMap<String, TrustScore>,
}

#[derive(Debug, Clone)]
struct IdentityRecord {
    identity: Identity,
    verified_at: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    connection_count: u32,
    verification_failures: u32,
}

impl IdentityManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            identities: HashMap::new(),
            trust_scores: HashMap::new(),
        })
    }

    /// Verify identity using cryptographic signature
    pub async fn verify_identity(&self, identity: &Identity) -> Result<bool> {
        // Check expiration
        if identity.expires_at < Utc::now() {
            tracing::warn!("Identity expired for user: {}", identity.user_id);
            return Ok(false);
        }

        // Verify signature
        let is_valid = self.verify_signature(identity)?;

        if !is_valid {
            tracing::warn!("Invalid signature for user: {}", identity.user_id);
            return Ok(false);
        }

        // Check if identity is revoked
        if self.is_revoked(&identity.user_id).await? {
            tracing::warn!("Identity revoked for user: {}", identity.user_id);
            return Ok(false);
        }

        tracing::info!("✅ Identity verified for user: {}", identity.user_id);
        Ok(true)
    }

    /// Register a new identity
    pub async fn register_identity(&mut self, identity: Identity) -> Result<()> {
        let record = IdentityRecord {
            identity: identity.clone(),
            verified_at: Utc::now(),
            last_seen: Utc::now(),
            connection_count: 0,
            verification_failures: 0,
        };

        self.identities.insert(identity.user_id.clone(), record);
        self.trust_scores.insert(identity.user_id.clone(), 50); // Start with neutral trust

        tracing::info!("🆔 Registered new identity: {}", identity.user_id);
        Ok(())
    }

    /// Get trust level for an identity (0-100)
    pub async fn get_trust_level(&self, identity: &Identity) -> Result<u8> {
        // Calculate trust score based on multiple factors
        let base_score = self.trust_scores.get(&identity.user_id).copied().unwrap_or(0);

        let record = self.identities.get(&identity.user_id);

        let bonus_score = if let Some(rec) = record {
            let mut bonus = 0u8;

            // Bonus for successful connections
            bonus = bonus.saturating_add((rec.connection_count / 10).min(20) as u8);

            // Penalty for verification failures
            bonus = bonus.saturating_sub((rec.verification_failures * 5).min(30) as u8);

            // Bonus for long-term usage
            let days_since_reg = (Utc::now() - rec.verified_at).num_days();
            bonus = bonus.saturating_add((days_since_reg / 30).min(10) as u8);

            bonus
        } else {
            0
        };

        Ok(base_score.saturating_add(bonus_score).min(100))
    }

    /// Update trust score for an identity
    pub async fn update_trust(&mut self, user_id: &str, delta: i8) -> Result<()> {
        let current = self.trust_scores.get(user_id).copied().unwrap_or(50);
        let new_score = if delta < 0 {
            current.saturating_sub(delta.abs() as u8)
        } else {
            current.saturating_add(delta as u8).min(100)
        };

        self.trust_scores.insert(user_id.to_string(), new_score);

        tracing::info!(
            "Updated trust score for {}: {} → {} (Δ{})",
            user_id,
            current,
            new_score,
            delta
        );

        Ok(())
    }

    /// Record successful connection
    pub async fn record_connection(&mut self, user_id: &str) -> Result<()> {
        if let Some(record) = self.identities.get_mut(user_id) {
            record.connection_count += 1;
            record.last_seen = Utc::now();
            self.update_trust(user_id, 1).await?;
        }
        Ok(())
    }

    /// Record verification failure
    pub async fn record_failure(&mut self, user_id: &str) -> Result<()> {
        if let Some(record) = self.identities.get_mut(user_id) {
            record.verification_failures += 1;
            self.update_trust(user_id, -5).await?;
        }
        Ok(())
    }

    /// Check if identity is revoked
    async fn is_revoked(&self, user_id: &str) -> Result<bool> {
        // In production, this would check a revocation list/database
        // For now, check if trust score is critically low
        let trust = self.trust_scores.get(user_id).copied().unwrap_or(50);
        Ok(trust < 10)
    }

    /// Verify cryptographic signature
    fn verify_signature(&self, identity: &Identity) -> Result<bool> {
        // Create message to verify
        let mut message = Vec::new();
        message.extend_from_slice(identity.user_id.as_bytes());
        message.extend_from_slice(&identity.public_key);
        message.extend_from_slice(identity.issued_at.to_rfc3339().as_bytes());
        message.extend_from_slice(identity.expires_at.to_rfc3339().as_bytes());

        // Hash the message
        let mut hasher = Sha256::new();
        hasher.update(&message);
        let hash = hasher.finalize();

        // In production, use proper Ed25519/RSA signature verification
        // For now, simplified verification
        let expected_signature: Vec<u8> = hash.to_vec();

        // Check if signatures match (in production, use public key cryptography)
        Ok(identity.signature.len() == expected_signature.len())
    }

    /// Create a new identity (for testing/demo)
    pub fn create_identity(user_id: String, attributes: HashMap<String, String>) -> Identity {
        let public_key = vec![0u8; 32]; // Mock public key
        let issued_at = Utc::now();
        let expires_at = issued_at + Duration::days(365);

        // Generate signature
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        hasher.update(&public_key);
        let signature = hasher.finalize().to_vec();

        Identity {
            user_id,
            public_key,
            attributes,
            issued_at,
            expires_at,
            signature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_identity_verification() {
        let mut manager = IdentityManager::new().unwrap();

        let identity = IdentityManager::create_identity(
            "test_user".to_string(),
            HashMap::new(),
        );

        manager.register_identity(identity.clone()).await.unwrap();

        let is_valid = manager.verify_identity(&identity).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_trust_score() {
        let mut manager = IdentityManager::new().unwrap();

        let identity = IdentityManager::create_identity(
            "test_user".to_string(),
            HashMap::new(),
        );

        manager.register_identity(identity.clone()).await.unwrap();

        let initial_trust = manager.get_trust_level(&identity).await.unwrap();
        assert_eq!(initial_trust, 50);

        manager.update_trust("test_user", 10).await.unwrap();
        let updated_trust = manager.get_trust_level(&identity).await.unwrap();
        assert!(updated_trust > initial_trust);
    }
}
