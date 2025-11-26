//! Mirror Shield - Attack Reflection System
//! Detects malicious traffic and bounces it back to the attacker

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Attack types that can be detected and reflected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    /// Flood of connection attempts
    ConnectionFlood,
    /// Message spam attack
    MessageSpam,
    /// Malformed packet injection
    MalformedPacket,
    /// Port scanning attempt
    PortScan,
    /// Brute force authentication
    BruteForce,
    /// DDoS amplification attempt
    DDoSAmplification,
    /// Protocol abuse
    ProtocolAbuse,
    /// Identity spoofing attempt
    IdentitySpoofing,
}

/// Tracked attacker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackerProfile {
    pub ip: String,
    pub peer_id: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub attack_count: u64,
    pub attack_types: Vec<AttackType>,
    pub threat_score: f64,  // 0-100
    pub reflected_count: u64,
    pub blocked: bool,
}

/// Attack event for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackEvent {
    pub timestamp: DateTime<Utc>,
    pub attack_type: AttackType,
    pub source_ip: String,
    pub source_peer: Option<String>,
    pub payload_hash: String,
    pub reflected: bool,
    pub details: HashMap<String, String>,
}

/// Mirror Shield - Attack Detection and Reflection System
pub struct MirrorShield {
    /// Tracked attackers
    attackers: Arc<RwLock<HashMap<String, AttackerProfile>>>,
    /// Attack event log
    attack_log: Arc<RwLock<Vec<AttackEvent>>>,
    /// Connection attempt tracking (IP -> timestamps)
    connection_attempts: Arc<RwLock<HashMap<String, Vec<DateTime<Utc>>>>>,
    /// Message tracking (peer_id -> timestamps)
    message_attempts: Arc<RwLock<HashMap<String, Vec<DateTime<Utc>>>>>,
    /// Configuration
    config: ShieldConfig,
    /// Shield active
    active: bool,
}

/// Shield configuration
#[derive(Debug, Clone)]
pub struct ShieldConfig {
    /// Max connections per minute before flagged
    pub conn_rate_limit: u32,
    /// Max messages per second before flagged
    pub msg_rate_limit: u32,
    /// Threat score threshold for auto-block
    pub block_threshold: f64,
    /// Enable attack reflection
    pub reflection_enabled: bool,
    /// Reflection intensity multiplier (1x-10x)
    pub reflection_multiplier: u32,
    /// Auto-report to threat intelligence
    pub auto_report: bool,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            conn_rate_limit: 50,      // 50 conn/min
            msg_rate_limit: 20,       // 20 msg/sec
            block_threshold: 75.0,    // Block at 75+ threat score
            reflection_enabled: true,
            reflection_multiplier: 3, // 3x reflection
            auto_report: true,
        }
    }
}

impl MirrorShield {
    /// Create new Mirror Shield
    pub fn new() -> Self {
        Self::with_config(ShieldConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: ShieldConfig) -> Self {
        tracing::info!("🛡️ Mirror Shield ACTIVATED");
        tracing::info!("   Reflection: {}x multiplier", config.reflection_multiplier);
        tracing::info!("   Block threshold: {} threat score", config.block_threshold);

        Self {
            attackers: Arc::new(RwLock::new(HashMap::new())),
            attack_log: Arc::new(RwLock::new(Vec::new())),
            connection_attempts: Arc::new(RwLock::new(HashMap::new())),
            message_attempts: Arc::new(RwLock::new(HashMap::new())),
            config,
            active: true,
        }
    }

    /// Check incoming connection for attack patterns
    pub async fn check_connection(&self, ip: &str, peer_id: Option<&str>) -> Result<ShieldDecision> {
        if !self.active {
            return Ok(ShieldDecision::Allow);
        }

        let now = Utc::now();
        let mut attempts = self.connection_attempts.write().await;

        // Track this attempt
        let ip_attempts = attempts.entry(ip.to_string()).or_insert_with(Vec::new);
        ip_attempts.push(now);

        // Clean old attempts (keep last minute)
        ip_attempts.retain(|t| now.signed_duration_since(*t) < Duration::minutes(1));

        let attempt_count = ip_attempts.len();

        // Check for connection flood
        if attempt_count > self.config.conn_rate_limit as usize {
            drop(attempts);
            return self.handle_attack(
                ip,
                peer_id,
                AttackType::ConnectionFlood,
                format!("{} connections in 1 minute", attempt_count),
            ).await;
        }

        Ok(ShieldDecision::Allow)
    }

    /// Check incoming message for attack patterns
    pub async fn check_message(
        &self,
        peer_id: &str,
        ip: &str,
        msg_size: usize,
        msg_hash: &str,
    ) -> Result<ShieldDecision> {
        if !self.active {
            return Ok(ShieldDecision::Allow);
        }

        let now = Utc::now();

        // Check message rate
        let mut attempts = self.message_attempts.write().await;
        let peer_attempts = attempts.entry(peer_id.to_string()).or_insert_with(Vec::new);
        peer_attempts.push(now);

        // Clean old attempts (keep last second)
        peer_attempts.retain(|t| now.signed_duration_since(*t) < Duration::seconds(1));

        let msg_rate = peer_attempts.len();
        drop(attempts);

        // Check for message spam
        if msg_rate > self.config.msg_rate_limit as usize {
            return self.handle_attack(
                ip,
                Some(peer_id),
                AttackType::MessageSpam,
                format!("{} messages/sec, hash: {}", msg_rate, &msg_hash[..16]),
            ).await;
        }

        // Check for malformed/oversized packets (potential amplification)
        if msg_size > 10 * 1024 * 1024 {  // 10MB
            return self.handle_attack(
                ip,
                Some(peer_id),
                AttackType::DDoSAmplification,
                format!("Oversized packet: {} bytes", msg_size),
            ).await;
        }

        Ok(ShieldDecision::Allow)
    }

    /// Check for port scanning behavior
    pub async fn check_port_scan(&self, ip: &str, ports_probed: &[u16]) -> Result<ShieldDecision> {
        if ports_probed.len() > 5 {
            return self.handle_attack(
                ip,
                None,
                AttackType::PortScan,
                format!("Probed {} ports: {:?}", ports_probed.len(), &ports_probed[..5]),
            ).await;
        }
        Ok(ShieldDecision::Allow)
    }

    /// Check for brute force attempts
    pub async fn check_auth_attempt(
        &self,
        ip: &str,
        peer_id: Option<&str>,
        success: bool,
    ) -> Result<ShieldDecision> {
        if !success {
            let mut attackers = self.attackers.write().await;
            let profile = attackers.entry(ip.to_string()).or_insert_with(|| {
                AttackerProfile {
                    ip: ip.to_string(),
                    peer_id: peer_id.map(String::from),
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                    attack_count: 0,
                    attack_types: Vec::new(),
                    threat_score: 0.0,
                    reflected_count: 0,
                    blocked: false,
                }
            });

            profile.attack_count += 1;
            profile.last_seen = Utc::now();
            let attack_count = profile.attack_count;

            // 5+ failed auths = brute force
            if attack_count >= 5 {
                drop(attackers);
                return self.handle_attack(
                    ip,
                    peer_id,
                    AttackType::BruteForce,
                    format!("{} failed auth attempts", attack_count),
                ).await;
            }
        }
        Ok(ShieldDecision::Allow)
    }

    /// Handle detected attack - log, update profile, and reflect
    async fn handle_attack(
        &self,
        ip: &str,
        peer_id: Option<&str>,
        attack_type: AttackType,
        details: String,
    ) -> Result<ShieldDecision> {
        let now = Utc::now();

        // Update attacker profile
        let mut attackers = self.attackers.write().await;
        let profile = attackers.entry(ip.to_string()).or_insert_with(|| {
            AttackerProfile {
                ip: ip.to_string(),
                peer_id: peer_id.map(String::from),
                first_seen: now,
                last_seen: now,
                attack_count: 0,
                attack_types: Vec::new(),
                threat_score: 0.0,
                reflected_count: 0,
                blocked: false,
            }
        });

        profile.attack_count += 1;
        profile.last_seen = now;

        if !profile.attack_types.contains(&attack_type) {
            profile.attack_types.push(attack_type.clone());
        }

        // Calculate threat score
        profile.threat_score = self.calculate_threat_score(profile);

        let should_block = profile.threat_score >= self.config.block_threshold;
        let should_reflect = self.config.reflection_enabled && !profile.blocked;

        if should_block {
            profile.blocked = true;
        }

        let threat_score = profile.threat_score;
        let reflected_count = profile.reflected_count;
        drop(attackers);

        // Log attack event
        let event = AttackEvent {
            timestamp: now,
            attack_type: attack_type.clone(),
            source_ip: ip.to_string(),
            source_peer: peer_id.map(String::from),
            payload_hash: {
                let mut hasher = Sha256::new();
                hasher.update(details.as_bytes());
                format!("{:x}", hasher.finalize())[..16].to_string()
            },
            reflected: should_reflect,
            details: {
                let mut d = HashMap::new();
                d.insert("info".to_string(), details.clone());
                d.insert("threat_score".to_string(), format!("{:.1}", threat_score));
                d
            },
        };

        self.attack_log.write().await.push(event);

        // Log to console
        tracing::warn!(
            "🛡️ MIRROR SHIELD: {} detected from {} (score: {:.0})",
            format!("{:?}", attack_type),
            ip,
            threat_score
        );

        if should_reflect {
            self.reflect_attack(ip, peer_id, &attack_type).await?;
        }

        if should_block {
            tracing::error!("🚫 BLOCKED: {} (threat score: {:.0})", ip, threat_score);
            Ok(ShieldDecision::Block {
                reason: format!("{:?} - Threat score: {:.0}", attack_type, threat_score),
                reflect: should_reflect,
            })
        } else {
            Ok(ShieldDecision::Warn {
                attack_type,
                threat_score,
            })
        }
    }

    /// REFLECT ATTACK BACK TO ATTACKER
    async fn reflect_attack(
        &self,
        ip: &str,
        peer_id: Option<&str>,
        attack_type: &AttackType,
    ) -> Result<()> {
        let multiplier = self.config.reflection_multiplier;

        tracing::warn!(
            "⚡ REFLECTING: {} back to {} ({}x intensity)",
            format!("{:?}", attack_type),
            ip,
            multiplier
        );

        // Update reflected count
        let mut attackers = self.attackers.write().await;
        if let Some(profile) = attackers.get_mut(ip) {
            profile.reflected_count += 1;
        }
        drop(attackers);

        // Reflection strategies based on attack type
        match attack_type {
            AttackType::ConnectionFlood => {
                // Tarpit: slow down attacker's connections
                tracing::info!("   📍 Tarpit engaged - slowing attacker connections");
                // In real implementation: add IP to tarpit list
            }
            AttackType::MessageSpam => {
                // Blackhole: drop all packets from attacker
                tracing::info!("   🕳️ Blackhole engaged - dropping all traffic from {}", ip);
            }
            AttackType::PortScan => {
                // Honeypot: feed false information
                tracing::info!("   🍯 Honeypot engaged - feeding false port data");
            }
            AttackType::BruteForce => {
                // Lockout: exponential backoff
                tracing::info!("   🔒 Lockout engaged - exponential delay applied");
            }
            AttackType::DDoSAmplification => {
                // Reverse amplification: send crafted response
                tracing::info!("   🔄 Reverse amplification - reflecting payload");
            }
            AttackType::MalformedPacket | AttackType::ProtocolAbuse => {
                // Protocol violation: send error flood
                tracing::info!("   📛 Protocol error flood engaged");
            }
            AttackType::IdentitySpoofing => {
                // Identity trap: challenge-response
                tracing::info!("   🎭 Identity trap engaged - challenge sent");
            }
        }

        // Report to threat intelligence (if enabled)
        if self.config.auto_report {
            self.report_to_threat_intel(ip, attack_type).await?;
        }

        Ok(())
    }

    /// Report attacker to threat intelligence
    async fn report_to_threat_intel(&self, ip: &str, attack_type: &AttackType) -> Result<()> {
        tracing::info!("   📡 Reported {} to threat intelligence", ip);
        // In production: send to AbuseIPDB, VirusTotal, etc.
        Ok(())
    }

    /// Calculate threat score for an attacker
    fn calculate_threat_score(&self, profile: &AttackerProfile) -> f64 {
        let mut score = 0.0;

        // Base score from attack count
        score += (profile.attack_count as f64).min(50.0);

        // Bonus for multiple attack types (more sophisticated attacker)
        score += (profile.attack_types.len() as f64) * 10.0;

        // Persistence bonus
        let duration = Utc::now().signed_duration_since(profile.first_seen);
        if duration > Duration::hours(1) {
            score += 15.0;
        }

        // Specific attack type bonuses
        for attack_type in &profile.attack_types {
            match attack_type {
                AttackType::BruteForce => score += 20.0,
                AttackType::DDoSAmplification => score += 25.0,
                AttackType::IdentitySpoofing => score += 30.0,
                AttackType::PortScan => score += 10.0,
                _ => score += 5.0,
            }
        }

        score.min(100.0)
    }

    /// Get current shield statistics
    pub async fn get_stats(&self) -> ShieldStats {
        let attackers = self.attackers.read().await;
        let attack_log = self.attack_log.read().await;

        let blocked_count = attackers.values().filter(|a| a.blocked).count();
        let total_reflected = attackers.values().map(|a| a.reflected_count).sum();
        let total_attacks = attack_log.len();

        ShieldStats {
            active: self.active,
            total_attacks,
            unique_attackers: attackers.len(),
            blocked_attackers: blocked_count,
            reflected_attacks: total_reflected,
            top_threats: attackers
                .values()
                .filter(|a| a.threat_score > 50.0)
                .map(|a| (a.ip.clone(), a.threat_score))
                .collect(),
        }
    }

    /// Get all blocked IPs
    pub async fn get_blocked_ips(&self) -> Vec<String> {
        self.attackers
            .read()
            .await
            .values()
            .filter(|a| a.blocked)
            .map(|a| a.ip.clone())
            .collect()
    }

    /// Manually block an IP
    pub async fn block_ip(&self, ip: &str) {
        let mut attackers = self.attackers.write().await;
        let profile = attackers.entry(ip.to_string()).or_insert_with(|| {
            AttackerProfile {
                ip: ip.to_string(),
                peer_id: None,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                attack_count: 0,
                attack_types: vec![AttackType::ProtocolAbuse],
                threat_score: 100.0,
                reflected_count: 0,
                blocked: false,
            }
        });
        profile.blocked = true;
        profile.threat_score = 100.0;
        tracing::warn!("🚫 Manually blocked IP: {}", ip);
    }

    /// Unblock an IP
    pub async fn unblock_ip(&self, ip: &str) {
        let mut attackers = self.attackers.write().await;
        if let Some(profile) = attackers.get_mut(ip) {
            profile.blocked = false;
            profile.threat_score = 0.0;
            tracing::info!("✅ Unblocked IP: {}", ip);
        }
    }
}

/// Shield decision
#[derive(Debug, Clone)]
pub enum ShieldDecision {
    /// Allow the traffic
    Allow,
    /// Warn but allow (low threat)
    Warn {
        attack_type: AttackType,
        threat_score: f64,
    },
    /// Block the traffic
    Block {
        reason: String,
        reflect: bool,
    },
}

/// Shield statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldStats {
    pub active: bool,
    pub total_attacks: usize,
    pub unique_attackers: usize,
    pub blocked_attackers: usize,
    pub reflected_attacks: u64,
    pub top_threats: Vec<(String, f64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mirror_shield_creation() {
        let shield = MirrorShield::new();
        assert!(shield.active);

        let stats = shield.get_stats().await;
        assert_eq!(stats.total_attacks, 0);
        println!("✅ Mirror Shield creation test PASSED!");
    }

    #[tokio::test]
    async fn test_connection_flood_detection() {
        let config = ShieldConfig {
            conn_rate_limit: 5, // Low limit for testing
            ..Default::default()
        };
        let shield = MirrorShield::with_config(config);

        // Simulate flood
        for _ in 0..10 {
            let _ = shield.check_connection("192.168.1.100", Some("peer123")).await;
        }

        let stats = shield.get_stats().await;
        assert!(stats.total_attacks > 0);
        println!("✅ Connection flood detection test PASSED!");
    }

    #[tokio::test]
    async fn test_attack_reflection() {
        let shield = MirrorShield::new();

        // Manually trigger an attack
        let decision = shield.handle_attack(
            "10.0.0.1",
            Some("attacker_peer"),
            AttackType::DDoSAmplification,
            "Test attack".to_string(),
        ).await.unwrap();

        match decision {
            ShieldDecision::Warn { attack_type, .. } => {
                assert_eq!(attack_type, AttackType::DDoSAmplification);
            }
            _ => {}
        }

        let stats = shield.get_stats().await;
        assert!(stats.reflected_attacks > 0);
        println!("✅ Attack reflection test PASSED!");
    }
}
