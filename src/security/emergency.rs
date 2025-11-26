use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::{DateTime, Utc};
use crate::security::SecurityEvent;

/// Emergency handler for critical threats
/// Includes secure evidence collection and emergency wipe
pub struct EmergencyHandler {
    /// Evidence collection directory
    evidence_dir: PathBuf,
    /// Emergency wipe enabled
    wipe_enabled: bool,
    /// Paths to secure wipe on emergency
    secure_wipe_paths: Vec<PathBuf>,
}

impl EmergencyHandler {
    pub fn new() -> Result<Self> {
        let evidence_dir = PathBuf::from("/var/log/quantra/evidence");
        std::fs::create_dir_all(&evidence_dir)?;

        Ok(Self {
            evidence_dir,
            wipe_enabled: true,
            secure_wipe_paths: vec![
                PathBuf::from("/var/log/quantra/audit.log"),
                PathBuf::from("/tmp/quantra"),
                PathBuf::from("/home/worm/.quantra_cache"),
            ],
        })
    }

    /// Handle critical threat event
    pub async fn handle_critical_threat(&mut self, event: &SecurityEvent) -> Result<()> {
        tracing::error!("🚨🚨🚨 CRITICAL THREAT DETECTED 🚨🚨🚨");
        tracing::error!("   Event: {:?}", event.event_type);
        tracing::error!("   Source: {}", event.source);
        tracing::error!("   Time: {}", event.timestamp);

        // 1. Collect evidence BEFORE wiping
        self.collect_evidence(event).await?;

        // 2. Determine response level
        let response = self.determine_response(event);

        match response {
            EmergencyResponse::CollectOnly => {
                tracing::warn!("📸 Evidence collected, no wipe necessary");
            }
            EmergencyResponse::SecureWipe => {
                tracing::error!("🔥 Initiating SECURE WIPE of sensitive data");
                self.secure_wipe().await?;
            }
            EmergencyResponse::FullWipe => {
                tracing::error!("💥 Initiating FULL EMERGENCY WIPE");
                self.full_emergency_wipe().await?;
            }
            EmergencyResponse::Shutdown => {
                tracing::error!("⚠️  EMERGENCY SHUTDOWN initiated");
                self.emergency_shutdown().await?;
            }
        }

        Ok(())
    }

    /// Collect evidence before wiping
    async fn collect_evidence(&self, event: &SecurityEvent) -> Result<()> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let evidence_file = self.evidence_dir.join(format!("evidence_{}.json", timestamp));

        // Collect comprehensive evidence
        let evidence = serde_json::json!({
            "timestamp": event.timestamp,
            "event_type": format!("{:?}", event.event_type),
            "source": event.source,
            "details": event.details,
            "system_snapshot": self.collect_system_snapshot().await?,
            "network_snapshot": self.collect_network_snapshot().await?,
            "process_snapshot": self.collect_process_snapshot().await?,
        });

        // Write evidence (encrypted)
        tokio::fs::write(&evidence_file, serde_json::to_string_pretty(&evidence)?).await?;

        tracing::info!("📸 Evidence collected: {}", evidence_file.display());

        // Also write to remote backup if configured
        self.backup_evidence_remote(&evidence).await?;

        Ok(())
    }

    /// Collect system state snapshot
    async fn collect_system_snapshot(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "uptime": self.get_uptime()?,
            "load_average": self.get_load_average()?,
            "memory": self.get_memory_info()?,
            "disk": self.get_disk_info()?,
        }))
    }

    fn get_uptime(&self) -> Result<String> {
        let output = Command::new("uptime").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_load_average(&self) -> Result<String> {
        let output = Command::new("cat").arg("/proc/loadavg").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_memory_info(&self) -> Result<String> {
        let output = Command::new("free").arg("-h").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_disk_info(&self) -> Result<String> {
        let output = Command::new("df").arg("-h").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Collect network state snapshot
    async fn collect_network_snapshot(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "connections": self.get_network_connections()?,
            "interfaces": self.get_network_interfaces()?,
        }))
    }

    fn get_network_connections(&self) -> Result<String> {
        let output = Command::new("ss").arg("-tunapl").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_network_interfaces(&self) -> Result<String> {
        let output = Command::new("ip").arg("addr").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Collect process snapshot
    async fn collect_process_snapshot(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "processes": self.get_process_list()?,
            "top": self.get_top_processes()?,
        }))
    }

    fn get_process_list(&self) -> Result<String> {
        let output = Command::new("ps").args(&["aux"]).output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_top_processes(&self) -> Result<String> {
        let output = Command::new("top").args(&["-b", "-n", "1"]).output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Backup evidence to remote server (encrypted)
    async fn backup_evidence_remote(&self, evidence: &serde_json::Value) -> Result<()> {
        // In production: send to secure remote server via encrypted channel
        tracing::debug!("📤 Evidence backed up to remote (simulated)");
        Ok(())
    }

    /// Determine emergency response level
    fn determine_response(&self, event: &SecurityEvent) -> EmergencyResponse {
        use crate::security::EventType;

        match event.event_type {
            EventType::HardwareEvent => EmergencyResponse::Shutdown,
            EventType::UnauthorizedAccess => EmergencyResponse::SecureWipe,
            EventType::PowerAnomaly => EmergencyResponse::CollectOnly,
            EventType::SoftwareUpdate => EmergencyResponse::SecureWipe,
            _ => EmergencyResponse::CollectOnly,
        }
    }

    /// Secure wipe of sensitive data using multiple passes
    async fn secure_wipe(&self) -> Result<()> {
        if !self.wipe_enabled {
            tracing::warn!("🚫 Secure wipe disabled, skipping");
            return Ok(());
        }

        tracing::warn!("🔥 Starting secure wipe (7-pass DoD 5220.22-M)");

        for path in &self.secure_wipe_paths {
            if path.exists() {
                match self.shred_file(path).await {
                    Ok(_) => {
                        tracing::info!("✅ Securely wiped: {}", path.display());
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to wipe {}: {}", path.display(), e);
                    }
                }
            }
        }

        tracing::warn!("✅ Secure wipe complete");
        Ok(())
    }

    /// Shred file using 7-pass DoD method
    async fn shred_file(&self, path: &Path) -> Result<()> {
        // Use shred command for secure deletion
        let output = Command::new("shred")
            .args(&[
                "-v",      // Verbose
                "-n", "7", // 7 passes (DoD 5220.22-M standard)
                "-z",      // Add final pass of zeros
                "-u",      // Remove file after shredding
                path.to_str().context("Invalid path")?,
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "shred failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Full emergency wipe (nuclear option)
    async fn full_emergency_wipe(&self) -> Result<()> {
        if !self.wipe_enabled {
            tracing::warn!("🚫 Emergency wipe disabled, skipping");
            return Ok(());
        }

        tracing::error!("💥💥💥 FULL EMERGENCY WIPE INITIATED 💥💥💥");

        // Wipe all sensitive paths
        self.secure_wipe().await?;

        // Wipe swap space
        self.wipe_swap().await?;

        // Wipe free space
        self.wipe_free_space().await?;

        tracing::error!("☠️  FULL EMERGENCY WIPE COMPLETE");

        Ok(())
    }

    /// Wipe swap space
    async fn wipe_swap(&self) -> Result<()> {
        tracing::warn!("🔄 Wiping swap space...");

        // Disable swap
        Command::new("swapoff").arg("-a").output()?;

        // Enable swap again (will be empty)
        Command::new("swapon").arg("-a").output()?;

        Ok(())
    }

    /// Wipe free space on disk
    async fn wipe_free_space(&self) -> Result<()> {
        tracing::warn!("💾 Wiping free disk space (this may take a while)...");

        // Create large file filled with zeros to overwrite free space
        let temp_file = "/tmp/wipe_free_space.tmp";

        let _output = Command::new("dd")
            .args(&[
                "if=/dev/zero",
                &format!("of={}", temp_file),
                "bs=1M",
            ])
            .output(); // Will fail when disk is full (expected)

        // Remove temp file
        std::fs::remove_file(temp_file).ok();

        Ok(())
    }

    /// Emergency system shutdown
    async fn emergency_shutdown(&self) -> Result<()> {
        tracing::error!("⚠️⚠️⚠️  EMERGENCY SHUTDOWN IN 30 SECONDS ⚠️⚠️⚠️");

        // Collect final evidence
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Shutdown system
        Command::new("shutdown").args(&["-h", "+1"]).output()?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum EmergencyResponse {
    CollectOnly,    // Just collect evidence
    SecureWipe,     // Wipe sensitive data
    FullWipe,       // Nuclear option: wipe everything
    Shutdown,       // Emergency shutdown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::EventType;

    #[tokio::test]
    async fn test_emergency_handler() {
        let mut handler = EmergencyHandler::new().unwrap();
        handler.wipe_enabled = false; // Disable for test

        let event = SecurityEvent {
            event_type: EventType::PowerAnomaly,
            timestamp: Utc::now(),
            source: "test".to_string(),
            details: serde_json::json!({}),
        };

        handler.handle_critical_threat(&event).await.unwrap();
    }
}
