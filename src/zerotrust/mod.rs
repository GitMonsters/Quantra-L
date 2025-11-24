pub mod identity;
pub mod policy;
pub mod vm_sandbox;
pub mod verification;
pub mod audit;

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Zero-Trust Security Context
/// Implements "never trust, always verify" principle
#[derive(Clone)]
pub struct ZeroTrustContext {
    identity_manager: Arc<RwLock<identity::IdentityManager>>,
    policy_engine: Arc<RwLock<policy::PolicyEngine>>,
    vm_manager: Arc<RwLock<vm_sandbox::VMManager>>,
    verifier: Arc<RwLock<verification::ContinuousVerifier>>,
    audit_log: Arc<RwLock<audit::AuditLogger>>,
}

/// Security Level for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    Untrusted,      // No verification
    Basic,          // Initial authentication only
    Verified,       // Continuous verification
    Privileged,     // Enhanced verification + VM isolation
    Critical,       // Maximum security + full isolation
}

/// Access Decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessDecision {
    Allow,
    Deny(String),           // Reason for denial
    AllowWithConditions(Vec<String>), // Conditions that must be met
}

/// Connection Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    pub peer_id: String,
    pub identity: identity::Identity,
    pub requested_resources: Vec<String>,
    pub client_metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

/// Active Connection with continuous verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureConnection {
    pub id: String,
    pub peer_id: String,
    pub identity: identity::Identity,
    pub security_level: SecurityLevel,
    pub vm_sandbox_id: Option<String>,
    pub granted_resources: Vec<String>,
    pub established_at: DateTime<Utc>,
    pub last_verified: DateTime<Utc>,
    pub verification_failures: u32,
}

impl ZeroTrustContext {
    pub fn new() -> Result<Self> {
        Ok(Self {
            identity_manager: Arc::new(RwLock::new(identity::IdentityManager::new()?)),
            policy_engine: Arc::new(RwLock::new(policy::PolicyEngine::new())),
            vm_manager: Arc::new(RwLock::new(vm_sandbox::VMManager::new()?)),
            verifier: Arc::new(RwLock::new(verification::ContinuousVerifier::new())),
            audit_log: Arc::new(RwLock::new(audit::AuditLogger::new()?)),
        })
    }

    /// Evaluate connection request using Zero-Trust principles
    pub async fn evaluate_connection(
        &self,
        request: ConnectionRequest,
    ) -> Result<AccessDecision> {
        // Step 1: Verify identity
        let identity_valid = self
            .identity_manager
            .read()
            .await
            .verify_identity(&request.identity)
            .await?;

        if !identity_valid {
            self.log_security_event(
                "identity_verification_failed",
                &request.peer_id,
                SecurityLevel::Untrusted,
            )
            .await?;
            return Ok(AccessDecision::Deny(
                "Identity verification failed".to_string(),
            ));
        }

        // Step 2: Check policies
        let policy_decision = self
            .policy_engine
            .read()
            .await
            .evaluate(&request.identity, &request.requested_resources)
            .await?;

        if let AccessDecision::Deny(reason) = policy_decision {
            self.log_security_event("policy_denied", &request.peer_id, SecurityLevel::Basic)
                .await?;
            return Ok(AccessDecision::Deny(reason));
        }

        // Step 3: Determine security level
        let security_level = self.determine_security_level(&request).await?;

        // Step 4: Apply VM isolation if required
        if security_level >= SecurityLevel::Privileged {
            let vm_available = self.vm_manager.read().await.has_capacity().await?;
            if !vm_available {
                return Ok(AccessDecision::Deny(
                    "VM isolation required but no capacity available".to_string(),
                ));
            }
        }

        self.log_security_event("access_granted", &request.peer_id, security_level)
            .await?;

        Ok(AccessDecision::Allow)
    }

    /// Establish secure connection with appropriate isolation
    pub async fn establish_connection(
        &self,
        request: ConnectionRequest,
    ) -> Result<SecureConnection> {
        let security_level = self.determine_security_level(&request).await?;

        // Create VM sandbox if needed
        let vm_sandbox_id = if security_level >= SecurityLevel::Privileged {
            let sandbox = self
                .vm_manager
                .write()
                .await
                .create_sandbox(&request.peer_id, security_level)
                .await?;
            Some(sandbox.id)
        } else {
            None
        };

        let connection = SecureConnection {
            id: uuid::Uuid::new_v4().to_string(),
            peer_id: request.peer_id.clone(),
            identity: request.identity.clone(),
            security_level,
            vm_sandbox_id,
            granted_resources: request.requested_resources.clone(),
            established_at: Utc::now(),
            last_verified: Utc::now(),
            verification_failures: 0,
        };

        // Start continuous verification
        self.verifier
            .write()
            .await
            .register_connection(connection.clone())
            .await?;

        self.log_security_event("connection_established", &request.peer_id, security_level)
            .await?;

        Ok(connection)
    }

    /// Continuously verify active connection
    pub async fn verify_connection(&self, connection_id: &str) -> Result<bool> {
        let verifier = self.verifier.read().await;
        verifier.verify(connection_id).await
    }

    /// Terminate connection and cleanup resources
    pub async fn terminate_connection(&self, connection_id: &str) -> Result<()> {
        let verifier = self.verifier.read().await;
        if let Some(connection) = verifier.get_connection(connection_id).await? {
            // Cleanup VM sandbox if exists
            if let Some(vm_id) = &connection.vm_sandbox_id {
                self.vm_manager
                    .write()
                    .await
                    .destroy_sandbox(vm_id)
                    .await?;
            }

            // Unregister from verification
            drop(verifier);
            self.verifier
                .write()
                .await
                .unregister_connection(connection_id)
                .await?;

            self.log_security_event(
                "connection_terminated",
                &connection.peer_id,
                connection.security_level,
            )
            .await?;
        }

        Ok(())
    }

    /// Get all active connections
    pub async fn get_active_connections(&self) -> Result<Vec<SecureConnection>> {
        self.verifier.read().await.get_all_connections().await
    }

    /// Determine appropriate security level based on request
    async fn determine_security_level(&self, request: &ConnectionRequest) -> Result<SecurityLevel> {
        // Check if requesting critical resources
        let has_critical_resources = request
            .requested_resources
            .iter()
            .any(|r| r.starts_with("critical/"));

        if has_critical_resources {
            return Ok(SecurityLevel::Critical);
        }

        // Check identity trust level
        let trust_level = self
            .identity_manager
            .read()
            .await
            .get_trust_level(&request.identity)
            .await?;

        match trust_level {
            0..=30 => Ok(SecurityLevel::Untrusted),
            31..=50 => Ok(SecurityLevel::Basic),
            51..=70 => Ok(SecurityLevel::Verified),
            71..=90 => Ok(SecurityLevel::Privileged),
            _ => Ok(SecurityLevel::Critical),
        }
    }

    /// Log security event
    async fn log_security_event(
        &self,
        event_type: &str,
        peer_id: &str,
        security_level: SecurityLevel,
    ) -> Result<()> {
        let event = audit::SecurityEvent {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            peer_id: peer_id.to_string(),
            security_level,
            details: HashMap::new(),
        };

        self.audit_log.write().await.log(event).await?;
        Ok(())
    }

    /// Get security statistics
    pub async fn get_stats(&self) -> Result<ZeroTrustStats> {
        let active_connections = self.get_active_connections().await?;
        let vm_stats = self.vm_manager.read().await.get_stats().await?;
        let audit_stats = self.audit_log.read().await.get_stats().await?;

        Ok(ZeroTrustStats {
            total_connections: active_connections.len(),
            by_security_level: self.count_by_level(&active_connections),
            active_vm_sandboxes: vm_stats.active_sandboxes,
            total_security_events: audit_stats.total_events,
            verification_failures: audit_stats.verification_failures,
        })
    }

    fn count_by_level(&self, connections: &[SecureConnection]) -> HashMap<SecurityLevel, usize> {
        let mut counts = HashMap::new();
        for conn in connections {
            *counts.entry(conn.security_level).or_insert(0) += 1;
        }
        counts
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroTrustStats {
    pub total_connections: usize,
    pub by_security_level: HashMap<SecurityLevel, usize>,
    pub active_vm_sandboxes: usize,
    pub total_security_events: usize,
    pub verification_failures: usize,
}
