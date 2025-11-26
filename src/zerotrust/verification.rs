//! Real Zero-Trust Continuous Verification
//!
//! Implements:
//! 1. Challenge-response re-authentication (Ed25519)
//! 2. Behavioral profiling and baseline tracking
//! 3. Statistical anomaly detection
//! 4. Dynamic trust score adjustment

use anyhow::{Result, Context, bail};
use std::collections::{HashMap, VecDeque};
use chrono::{Utc, DateTime, Duration, Timelike};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey, SigningKey};
use sha2::{Sha256, Digest};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use crate::zerotrust::SecureConnection;

/// Maximum events to track for behavioral analysis
const MAX_BEHAVIOR_EVENTS: usize = 1000;
/// Anomaly threshold (z-score) for flagging suspicious behavior
const ANOMALY_Z_THRESHOLD: f64 = 2.5;
/// Challenge validity window
const CHALLENGE_VALIDITY_SECS: i64 = 30;

/// Event types for behavioral tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorEvent {
    MessageSent { bytes: u64, timestamp: DateTime<Utc> },
    MessageReceived { bytes: u64, timestamp: DateTime<Utc> },
    ResourceAccess { resource: String, timestamp: DateTime<Utc> },
    AuthSuccess { timestamp: DateTime<Utc> },
    AuthFailure { reason: String, timestamp: DateTime<Utc> },
    AnomalyDetected { score: f64, reason: String, timestamp: DateTime<Utc> },
}

/// Behavioral profile for a connection
#[derive(Debug, Clone)]
pub struct BehaviorProfile {
    /// Rolling window of events
    events: VecDeque<BehaviorEvent>,

    /// Access pattern statistics
    pub total_messages: u64,
    pub total_bytes: u64,
    pub resources_accessed: HashMap<String, u32>,

    /// Hourly activity histogram (24 hours)
    pub hourly_activity: [u32; 24],

    /// Statistical baselines (computed from history)
    pub baseline_msgs_per_hour: f64,
    pub baseline_bytes_per_hour: f64,
    pub baseline_stddev_msgs: f64,
    pub baseline_stddev_bytes: f64,

    /// Anomaly tracking
    pub anomaly_score: f64,
    pub consecutive_anomalies: u32,
    pub total_anomalies: u32,
    pub last_anomaly: Option<DateTime<Utc>>,

    /// Profile creation time
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl BehaviorProfile {
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_BEHAVIOR_EVENTS),
            total_messages: 0,
            total_bytes: 0,
            resources_accessed: HashMap::new(),
            hourly_activity: [0; 24],
            baseline_msgs_per_hour: 0.0,
            baseline_bytes_per_hour: 0.0,
            baseline_stddev_msgs: 10.0,  // Initial generous stddev
            baseline_stddev_bytes: 10000.0,
            anomaly_score: 0.0,
            consecutive_anomalies: 0,
            total_anomalies: 0,
            last_anomaly: None,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Record a behavioral event
    pub fn record_event(&mut self, event: BehaviorEvent) {
        // Maintain rolling window
        if self.events.len() >= MAX_BEHAVIOR_EVENTS {
            self.events.pop_front();
        }

        // Update statistics based on event type
        match &event {
            BehaviorEvent::MessageSent { bytes, timestamp } |
            BehaviorEvent::MessageReceived { bytes, timestamp } => {
                self.total_messages += 1;
                self.total_bytes += bytes;
                self.hourly_activity[timestamp.hour() as usize] += 1;
            }
            BehaviorEvent::ResourceAccess { resource, .. } => {
                *self.resources_accessed.entry(resource.clone()).or_insert(0) += 1;
            }
            BehaviorEvent::AnomalyDetected { score, .. } => {
                self.anomaly_score = *score;
                self.consecutive_anomalies += 1;
                self.total_anomalies += 1;
                self.last_anomaly = Some(Utc::now());
            }
            BehaviorEvent::AuthSuccess { .. } => {
                // Reset consecutive anomalies on successful auth
                self.consecutive_anomalies = 0;
            }
            BehaviorEvent::AuthFailure { .. } => {
                self.consecutive_anomalies += 1;
            }
        }

        self.events.push_back(event);
        self.last_updated = Utc::now();

        // Recompute baselines periodically
        if self.events.len() % 100 == 0 {
            self.recompute_baselines();
        }
    }

    /// Recompute statistical baselines from event history
    fn recompute_baselines(&mut self) {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        // Count messages and bytes in the last hour
        let (msgs_last_hour, bytes_last_hour): (u64, u64) = self.events.iter()
            .filter_map(|e| match e {
                BehaviorEvent::MessageSent { bytes, timestamp } |
                BehaviorEvent::MessageReceived { bytes, timestamp }
                    if *timestamp > one_hour_ago => Some((1u64, *bytes)),
                _ => None,
            })
            .fold((0, 0), |(m, b), (dm, db)| (m + dm, b + db));

        // Exponential moving average for baselines
        let alpha = 0.1;  // Smoothing factor
        self.baseline_msgs_per_hour =
            self.baseline_msgs_per_hour * (1.0 - alpha) + (msgs_last_hour as f64) * alpha;
        self.baseline_bytes_per_hour =
            self.baseline_bytes_per_hour * (1.0 - alpha) + (bytes_last_hour as f64) * alpha;

        // Compute standard deviation from hourly variance
        let hours_active = self.hourly_activity.iter().filter(|&&x| x > 0).count().max(1);
        let mean_hourly = self.total_messages as f64 / hours_active as f64;
        let variance: f64 = self.hourly_activity.iter()
            .map(|&x| (x as f64 - mean_hourly).powi(2))
            .sum::<f64>() / 24.0;
        self.baseline_stddev_msgs = variance.sqrt().max(1.0);
    }

    /// Check current behavior against baseline, return anomaly score
    pub fn detect_anomaly(&self, current_msgs_per_hour: f64, current_bytes_per_hour: f64) -> (f64, Vec<String>) {
        let mut reasons = Vec::new();
        let mut max_z_score: f64 = 0.0;

        // Z-score for message rate
        if self.baseline_stddev_msgs > 0.0 {
            let z_msgs = (current_msgs_per_hour - self.baseline_msgs_per_hour).abs()
                / self.baseline_stddev_msgs;
            if z_msgs > ANOMALY_Z_THRESHOLD {
                reasons.push(format!("Message rate anomaly: z={:.2} (current={:.1}/hr, baseline={:.1}/hr)",
                    z_msgs, current_msgs_per_hour, self.baseline_msgs_per_hour));
                max_z_score = max_z_score.max(z_msgs);
            }
        }

        // Z-score for byte rate
        if self.baseline_stddev_bytes > 0.0 {
            let z_bytes = (current_bytes_per_hour - self.baseline_bytes_per_hour).abs()
                / self.baseline_stddev_bytes;
            if z_bytes > ANOMALY_Z_THRESHOLD {
                reasons.push(format!("Byte rate anomaly: z={:.2} (current={:.1}/hr, baseline={:.1}/hr)",
                    z_bytes, current_bytes_per_hour, self.baseline_bytes_per_hour));
                max_z_score = max_z_score.max(z_bytes);
            }
        }

        // Check for unusual access hours
        let current_hour = Utc::now().hour() as usize;
        if self.hourly_activity[current_hour] == 0 && self.total_messages > 100 {
            // First activity in this hour after significant history
            reasons.push(format!("Unusual access hour: {} (no prior activity)", current_hour));
            max_z_score = max_z_score.max(1.5);
        }

        // Normalize to 0-1 range
        let anomaly_score = (max_z_score / 5.0).min(1.0);

        (anomaly_score, reasons)
    }
}

/// Challenge for cryptographic re-authentication
#[derive(Debug, Clone)]
pub struct VerificationChallenge {
    /// Random nonce (32 bytes)
    pub nonce: [u8; 32],
    /// When challenge was issued
    pub issued_at: DateTime<Utc>,
    /// When challenge expires
    pub expires_at: DateTime<Utc>,
    /// Target peer ID
    pub peer_id: String,
    /// Expected public key for verification
    pub expected_public_key: Vec<u8>,
}

impl VerificationChallenge {
    pub fn new(peer_id: String, public_key: Vec<u8>) -> Self {
        let mut nonce = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut nonce);

        let now = Utc::now();
        Self {
            nonce,
            issued_at: now,
            expires_at: now + Duration::seconds(CHALLENGE_VALIDITY_SECS),
            peer_id,
            expected_public_key: public_key,
        }
    }

    /// Check if challenge is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }

    /// Get the message that must be signed
    pub fn get_sign_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.nonce);
        message.extend_from_slice(self.peer_id.as_bytes());
        message.extend_from_slice(self.issued_at.timestamp().to_le_bytes().as_slice());
        message
    }

    /// Verify a response signature
    pub fn verify_response(&self, signature: &[u8]) -> Result<bool> {
        if !self.is_valid() {
            bail!("Challenge expired");
        }

        if signature.len() != 64 {
            bail!("Invalid signature length: {}", signature.len());
        }

        if self.expected_public_key.len() != 32 {
            bail!("Invalid public key length");
        }

        let public_key_bytes: [u8; 32] = self.expected_public_key[..32]
            .try_into()
            .context("Invalid public key format")?;

        let public_key = VerifyingKey::from_bytes(&public_key_bytes)
            .context("Invalid Ed25519 public key")?;

        let signature_bytes: [u8; 64] = signature[..64]
            .try_into()
            .context("Invalid signature format")?;

        let signature = Signature::from_bytes(&signature_bytes);
        let message = self.get_sign_message();

        match public_key.verify(&message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Verification result with detailed status
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub success: bool,
    pub challenge_passed: bool,
    pub behavior_ok: bool,
    pub anomaly_score: f64,
    pub anomaly_reasons: Vec<String>,
    pub trust_delta: i8,
    pub new_security_level: Option<crate::zerotrust::SecurityLevel>,
    pub timestamp: DateTime<Utc>,
}

/// Real Continuous Verifier with challenge-response and behavioral analysis
pub struct ContinuousVerifier {
    /// Active connections
    connections: HashMap<String, SecureConnection>,
    /// Behavioral profiles per peer
    behaviors: HashMap<String, BehaviorProfile>,
    /// Pending verification challenges
    pending_challenges: HashMap<String, VerificationChallenge>,
    /// Verification interval
    verification_interval: Duration,
    /// Anomaly threshold for triggering re-auth
    anomaly_threshold: f64,
}

impl ContinuousVerifier {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            behaviors: HashMap::new(),
            pending_challenges: HashMap::new(),
            verification_interval: Duration::minutes(5),
            anomaly_threshold: 0.7,
        }
    }

    /// Register a new connection for continuous verification
    pub async fn register_connection(&mut self, connection: SecureConnection) -> Result<()> {
        let peer_id = connection.peer_id.clone();
        let conn_id = connection.id.clone();

        // Create behavioral profile if new peer
        self.behaviors.entry(peer_id.clone())
            .or_insert_with(BehaviorProfile::new);

        self.connections.insert(conn_id.clone(), connection);

        tracing::info!("🔐 Registered connection {} for continuous verification", conn_id);
        Ok(())
    }

    /// Unregister a connection
    pub async fn unregister_connection(&mut self, connection_id: &str) -> Result<()> {
        self.connections.remove(connection_id);
        self.pending_challenges.remove(connection_id);
        tracing::info!("🔓 Unregistered connection {} from verification", connection_id);
        Ok(())
    }

    /// Issue a verification challenge for a connection
    pub fn issue_challenge(&mut self, connection_id: &str) -> Result<VerificationChallenge> {
        let conn = self.connections.get(connection_id)
            .context("Connection not found")?;

        let challenge = VerificationChallenge::new(
            conn.peer_id.clone(),
            conn.identity.public_key.clone(),
        );

        tracing::info!("🎲 Issued verification challenge for {}: nonce={}",
            connection_id,
            hex::encode(&challenge.nonce[..8]));

        self.pending_challenges.insert(connection_id.to_string(), challenge.clone());
        Ok(challenge)
    }

    /// Verify a challenge response
    pub async fn verify_challenge_response(
        &mut self,
        connection_id: &str,
        signature: &[u8]
    ) -> Result<VerificationResult> {
        let challenge = self.pending_challenges.remove(connection_id)
            .context("No pending challenge for this connection")?;

        let conn = self.connections.get_mut(connection_id)
            .context("Connection not found")?;

        let behavior = self.behaviors.get_mut(&conn.peer_id)
            .context("No behavioral profile")?;

        // Verify the cryptographic response
        let challenge_passed = challenge.verify_response(signature)?;

        if challenge_passed {
            behavior.record_event(BehaviorEvent::AuthSuccess {
                timestamp: Utc::now()
            });
            conn.last_verified = Utc::now();
            conn.verification_failures = 0;

            tracing::info!("✅ Challenge-response verification PASSED for {}", connection_id);
        } else {
            behavior.record_event(BehaviorEvent::AuthFailure {
                reason: "Invalid signature".to_string(),
                timestamp: Utc::now()
            });
            conn.verification_failures += 1;

            tracing::warn!("❌ Challenge-response verification FAILED for {}", connection_id);
        }

        // Calculate trust delta based on result
        let trust_delta = if challenge_passed { 5 } else { -15 };

        Ok(VerificationResult {
            success: challenge_passed,
            challenge_passed,
            behavior_ok: true,
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            trust_delta,
            new_security_level: None,
            timestamp: Utc::now(),
        })
    }

    /// Record behavioral event for a connection
    pub fn record_behavior(&mut self, connection_id: &str, event: BehaviorEvent) -> Result<()> {
        let conn = self.connections.get(connection_id)
            .context("Connection not found")?;

        if let Some(behavior) = self.behaviors.get_mut(&conn.peer_id) {
            behavior.record_event(event);
        }

        Ok(())
    }

    /// Full verification: check timing, behavior, and optionally issue challenge
    pub async fn verify(&mut self, connection_id: &str) -> Result<VerificationResult> {
        let conn = self.connections.get(connection_id)
            .context("Connection not found")?;

        let time_since_last = Utc::now() - conn.last_verified;
        let needs_reauth = time_since_last > self.verification_interval;

        // Get behavioral analysis
        let (anomaly_score, anomaly_reasons) = if let Some(behavior) = self.behaviors.get(&conn.peer_id) {
            // Calculate current rates (simplified - would need actual tracking)
            let hours_active = ((Utc::now() - behavior.created_at).num_minutes() as f64 / 60.0).max(1.0);
            let current_msgs_per_hour = behavior.total_messages as f64 / hours_active;
            let current_bytes_per_hour = behavior.total_bytes as f64 / hours_active;

            behavior.detect_anomaly(current_msgs_per_hour, current_bytes_per_hour)
        } else {
            (0.0, vec![])
        };

        let behavior_ok = anomaly_score < self.anomaly_threshold;

        // Record anomaly if detected
        if !behavior_ok {
            if let Some(behavior) = self.behaviors.get_mut(&conn.peer_id) {
                behavior.record_event(BehaviorEvent::AnomalyDetected {
                    score: anomaly_score,
                    reason: anomaly_reasons.join("; "),
                    timestamp: Utc::now(),
                });
            }
            tracing::warn!("⚠️ Anomaly detected for {}: score={:.2}, reasons={:?}",
                connection_id, anomaly_score, anomaly_reasons);
        }

        // Determine if we need a challenge
        let force_challenge = !behavior_ok || conn.verification_failures > 0;

        // Calculate trust adjustment
        let trust_delta = if behavior_ok {
            if needs_reauth { 0 } else { 1 }  // Small positive for good behavior
        } else {
            -((anomaly_score * 10.0) as i8)  // Negative proportional to anomaly
        };

        // Determine new security level based on behavior
        let new_security_level = if anomaly_score > 0.9 {
            Some(crate::zerotrust::SecurityLevel::Untrusted)
        } else if anomaly_score > 0.5 {
            Some(crate::zerotrust::SecurityLevel::Basic)
        } else {
            None  // No change
        };

        let success = !needs_reauth && behavior_ok && !force_challenge;

        if needs_reauth || force_challenge {
            tracing::info!("🔄 Re-authentication required for {}: needs_reauth={}, force={}",
                connection_id, needs_reauth, force_challenge);
        }

        Ok(VerificationResult {
            success,
            challenge_passed: !needs_reauth,  // Will be updated after challenge
            behavior_ok,
            anomaly_score,
            anomaly_reasons,
            trust_delta,
            new_security_level,
            timestamp: Utc::now(),
        })
    }

    /// Get connection by ID
    pub async fn get_connection(&self, connection_id: &str) -> Result<Option<SecureConnection>> {
        Ok(self.connections.get(connection_id).cloned())
    }

    /// Get all active connections
    pub async fn get_all_connections(&self) -> Result<Vec<SecureConnection>> {
        Ok(self.connections.values().cloned().collect())
    }

    /// Get behavioral profile for a peer
    pub fn get_behavior_profile(&self, peer_id: &str) -> Option<&BehaviorProfile> {
        self.behaviors.get(peer_id)
    }

    /// Get verification statistics
    pub fn get_stats(&self) -> VerificationStats {
        let total_anomalies: u32 = self.behaviors.values()
            .map(|b| b.total_anomalies)
            .sum();

        let avg_anomaly_score: f64 = if self.behaviors.is_empty() {
            0.0
        } else {
            self.behaviors.values()
                .map(|b| b.anomaly_score)
                .sum::<f64>() / self.behaviors.len() as f64
        };

        VerificationStats {
            active_connections: self.connections.len(),
            tracked_peers: self.behaviors.len(),
            pending_challenges: self.pending_challenges.len(),
            total_anomalies,
            avg_anomaly_score,
        }
    }
}

/// Statistics about the verification system
#[derive(Debug, Clone)]
pub struct VerificationStats {
    pub active_connections: usize,
    pub tracked_peers: usize,
    pub pending_challenges: usize,
    pub total_anomalies: u32,
    pub avg_anomaly_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zerotrust::{identity::IdentityManager, SecurityLevel};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_challenge_response() {
        // Generate a keypair
        let mut csprng = rand::rngs::OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let public_key = signing_key.verifying_key().to_bytes().to_vec();

        // Create challenge
        let challenge = VerificationChallenge::new(
            "test-peer".to_string(),
            public_key,
        );

        // Sign the challenge
        let message = challenge.get_sign_message();
        let signature = signing_key.sign(&message);

        // Verify
        let result = challenge.verify_response(&signature.to_bytes()).unwrap();
        assert!(result, "Valid signature should verify");
    }

    #[tokio::test]
    async fn test_challenge_response_invalid() {
        let mut csprng = rand::rngs::OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let public_key = signing_key.verifying_key().to_bytes().to_vec();

        let challenge = VerificationChallenge::new(
            "test-peer".to_string(),
            public_key,
        );

        // Wrong signature (all zeros)
        let bad_signature = [0u8; 64];
        let result = challenge.verify_response(&bad_signature).unwrap();
        assert!(!result, "Invalid signature should fail");
    }

    #[test]
    fn test_behavioral_profiling() {
        let mut profile = BehaviorProfile::new();

        // Record some events
        for _i in 0..50 {
            profile.record_event(BehaviorEvent::MessageSent {
                bytes: 1000,
                timestamp: Utc::now(),
            });
        }

        assert_eq!(profile.total_messages, 50);
        assert_eq!(profile.total_bytes, 50000);

        // Recompute baselines to establish normal behavior
        profile.recompute_baselines();

        // Check anomaly detection with rate matching baseline (should be low)
        // Using values close to what was established
        let (score, _reasons) = profile.detect_anomaly(
            profile.baseline_msgs_per_hour,
            profile.baseline_bytes_per_hour
        );
        assert!(score < 0.7, "Behavior matching baseline should have low anomaly score: got {}", score);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut profile = BehaviorProfile::new();

        // Establish baseline with low activity
        for _ in 0..100 {
            profile.record_event(BehaviorEvent::MessageSent {
                bytes: 100,
                timestamp: Utc::now(),
            });
        }
        profile.recompute_baselines();

        // Check with extremely high rate (should trigger anomaly)
        let (score, reasons) = profile.detect_anomaly(10000.0, 10000000.0);
        assert!(score > 0.0 || !reasons.is_empty(),
            "Abnormal spike should be detected");
    }

    #[tokio::test]
    async fn test_continuous_verifier_registration() {
        let mut verifier = ContinuousVerifier::new();

        let identity = IdentityManager::create_identity(
            "test-peer".to_string(),
            HashMap::new(),
        );

        let connection = SecureConnection {
            id: "conn-1".to_string(),
            peer_id: "test-peer".to_string(),
            identity,
            security_level: SecurityLevel::Basic,
            vm_sandbox_id: None,
            granted_resources: vec![],
            established_at: Utc::now(),
            last_verified: Utc::now(),
            verification_failures: 0,
        };

        verifier.register_connection(connection).await.unwrap();

        let stats = verifier.get_stats();
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.tracked_peers, 1);
    }
}
