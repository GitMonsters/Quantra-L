use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::zerotrust::SecurityLevel;

/// Security Event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub peer_id: String,
    pub security_level: SecurityLevel,
    pub details: HashMap<String, String>,
}

/// Audit Logger
pub struct AuditLogger {
    events: Vec<SecurityEvent>,
}

#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_events: usize,
    pub verification_failures: usize,
}

impl AuditLogger {
    pub fn new() -> Result<Self> {
        Ok(Self { events: Vec::new() })
    }

    pub async fn log(&mut self, event: SecurityEvent) -> Result<()> {
        tracing::info!(
            "📋 Audit: {} - {} (level: {:?})",
            event.event_type,
            event.peer_id,
            event.security_level
        );
        self.events.push(event);
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<AuditStats> {
        let verification_failures = self
            .events
            .iter()
            .filter(|e| e.event_type.contains("failed") || e.event_type.contains("denied"))
            .count();

        Ok(AuditStats {
            total_events: self.events.len(),
            verification_failures,
        })
    }
}
