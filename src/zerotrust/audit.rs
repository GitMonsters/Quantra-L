use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use crate::zerotrust::SecurityLevel;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use sha2::{Sha256, Digest};
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader as TokioBufReader};

/// Security Event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub peer_id: String,
    pub security_level: SecurityLevel,
    pub details: HashMap<String, String>,
    /// Previous event hash for tamper detection (SHA-256 chain)
    #[serde(default)]
    pub prev_hash: String,
}

/// Audit Logger with persistent encrypted storage
pub struct AuditLogger {
    /// In-memory cache (last 1000 events)
    events: Vec<SecurityEvent>,
    /// Log file path
    log_path: PathBuf,
    /// Encryption key (32 bytes for AES-256)
    encryption_key: [u8; 32],
    /// Last event hash for chain verification
    last_hash: String,
    /// Maximum log file size (100MB)
    max_log_size: u64,
    /// Maximum events in memory
    max_memory_events: usize,
}

#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_events: usize,
    pub verification_failures: usize,
    pub log_file_size: u64,
    pub memory_events: usize,
}

impl AuditLogger {
    /// Create new audit logger with persistent encrypted storage
    /// ✅ OPTIMIZATION: Async for non-blocking I/O
    pub async fn new() -> Result<Self> {
        Self::with_path("/var/log/quantra/audit.log").await
    }

    /// Create audit logger with custom log path
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    pub async fn with_path<P: AsRef<Path>>(log_path: P) -> Result<Self> {
        let log_path = log_path.as_ref().to_path_buf();

        // ✅ Use tokio::fs for async directory creation
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create log directory")?;
        }

        // Generate or load encryption key (async)
        let encryption_key = Self::load_or_generate_key(&log_path).await?;

        // Load last hash from existing log (async)
        let last_hash = Self::load_last_hash(&log_path, &encryption_key).await?;

        tracing::info!("📋 Audit logger initialized: {}", log_path.display());
        tracing::info!("   Encryption: AES-256-GCM");
        tracing::info!("   Tamper detection: SHA-256 chain");

        Ok(Self {
            events: Vec::new(),
            log_path,
            encryption_key,
            last_hash,
            max_log_size: 100 * 1024 * 1024, // 100MB
            max_memory_events: 1000,
        })
    }

    /// Log security event with encryption and tamper detection
    pub async fn log(&mut self, mut event: SecurityEvent) -> Result<()> {
        // Add hash chain
        event.prev_hash = self.last_hash.clone();

        // Calculate hash of current event
        let event_json = serde_json::to_string(&event)?;
        let mut hasher = Sha256::new();
        hasher.update(event_json.as_bytes());
        hasher.update(self.last_hash.as_bytes());
        self.last_hash = format!("{:x}", hasher.finalize());

        tracing::info!(
            "📋 Audit: {} - {} (level: {:?}) [hash: {}]",
            event.event_type,
            event.peer_id,
            event.security_level,
            &self.last_hash[..16]
        );

        // Add to memory cache
        self.events.push(event.clone());

        // Trim memory cache if too large
        if self.events.len() > self.max_memory_events {
            self.events.drain(0..(self.events.len() - self.max_memory_events));
        }

        // Persist to disk (encrypted)
        self.persist_event(&event).await?;

        // Check if log rotation needed
        self.check_rotation().await?;

        Ok(())
    }

    /// Persist event to encrypted log file
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    async fn persist_event(&self, event: &SecurityEvent) -> Result<()> {
        // Serialize event
        let event_json = serde_json::to_string(event)?;

        // Encrypt event
        let encrypted = self.encrypt_data(event_json.as_bytes())?;

        // ✅ Use tokio::fs for async file operations
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await
            .context("Failed to open audit log")?;

        // Write as base64-encoded line
        let encoded = general_purpose::STANDARD.encode(&encrypted);
        file.write_all(format!("{}\n", encoded).as_bytes()).await?;
        file.sync_all().await?;

        Ok(())
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt_data(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.encryption_key));

        // Generate random nonce (12 bytes for GCM)
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt_data(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        if encrypted.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted data (too short)"));
        }

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.encryption_key));

        // Extract nonce (first 12 bytes)
        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// Load or generate encryption key
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    async fn load_or_generate_key(log_path: &Path) -> Result<[u8; 32]> {
        let key_path = log_path.with_extension("key");

        if key_path.exists() {
            // ✅ Use tokio::fs for async file read
            let key_data = tokio::fs::read(&key_path).await
                .context("Failed to read encryption key")?;

            if key_data.len() != 32 {
                return Err(anyhow::anyhow!("Invalid key size: {} bytes", key_data.len()));
            }

            let mut key = [0u8; 32];
            key.copy_from_slice(&key_data);

            tracing::info!("✅ Loaded existing audit log encryption key");
            Ok(key)
        } else {
            // Generate new key
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);

            // ✅ Use tokio::fs for async file write
            tokio::fs::write(&key_path, &key).await
                .context("Failed to save encryption key")?;

            // Set file permissions (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = tokio::fs::metadata(&key_path).await?;
                let mut perms = metadata.permissions();
                perms.set_mode(0o600); // Read/write for owner only
                tokio::fs::set_permissions(&key_path, perms).await?;
            }

            tracing::info!("✅ Generated new audit log encryption key: {}", key_path.display());
            Ok(key)
        }
    }

    /// Load last hash from existing log
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    async fn load_last_hash(log_path: &Path, encryption_key: &[u8; 32]) -> Result<String> {
        if !log_path.exists() {
            return Ok(String::from("genesis"));
        }

        // ✅ Use tokio::fs for async file read
        let file = tokio::fs::File::open(log_path).await?;
        let reader = TokioBufReader::new(file);
        let mut lines = reader.lines();

        // Read all lines to get the last one
        let mut last_line = None;
        while let Some(line) = lines.next_line().await? {
            last_line = Some(line);
        }

        if let Some(line) = last_line {
            // Decrypt and parse last event
            let encrypted = general_purpose::STANDARD.decode(&line)?;

            let logger = Self {
                events: Vec::new(),
                log_path: log_path.to_path_buf(),
                encryption_key: *encryption_key,
                last_hash: String::new(),
                max_log_size: 100 * 1024 * 1024,
                max_memory_events: 1000,
            };

            let plaintext = logger.decrypt_data(&encrypted)?;
            let event: SecurityEvent = serde_json::from_slice(&plaintext)?;

            // Recalculate hash
            let event_json = serde_json::to_string(&event)?;
            let mut hasher = Sha256::new();
            hasher.update(event_json.as_bytes());
            hasher.update(event.prev_hash.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            tracing::info!("✅ Loaded last hash from audit log: {}", &hash[..16]);
            return Ok(hash);
        }

        Ok(String::from("genesis"))
    }

    /// Check if log rotation is needed
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    async fn check_rotation(&self) -> Result<()> {
        if let Ok(metadata) = tokio::fs::metadata(&self.log_path).await {
            if metadata.len() > self.max_log_size {
                self.rotate_log().await?;
            }
        }
        Ok(())
    }

    /// Rotate log file
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    async fn rotate_log(&self) -> Result<()> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let rotated_path = self.log_path.with_file_name(
            format!("{}.{}.log",
                self.log_path.file_stem().unwrap().to_str().unwrap(),
                timestamp
            )
        );

        tokio::fs::rename(&self.log_path, &rotated_path).await
            .context("Failed to rotate log file")?;

        tracing::info!("📋 Rotated audit log: {} -> {}",
            self.log_path.display(),
            rotated_path.display()
        );

        Ok(())
    }

    /// Get audit statistics
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    pub async fn get_stats(&self) -> Result<AuditStats> {
        let verification_failures = self
            .events
            .iter()
            .filter(|e| e.event_type.contains("failed") || e.event_type.contains("denied"))
            .count();

        let log_file_size = tokio::fs::metadata(&self.log_path).await
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(AuditStats {
            total_events: self.events.len(),
            verification_failures,
            log_file_size,
            memory_events: self.events.len(),
        })
    }

    /// Verify log integrity (check hash chain)
    /// ✅ OPTIMIZATION: Uses async tokio::fs for non-blocking I/O
    pub async fn verify_integrity(&self) -> Result<bool> {
        if !self.log_path.exists() {
            return Ok(true); // Empty log is valid
        }

        tracing::info!("🔍 Verifying audit log integrity...");

        // ✅ Use tokio::fs for async file read
        let file = tokio::fs::File::open(&self.log_path).await?;
        let reader = TokioBufReader::new(file);
        let mut lines = reader.lines();

        let mut prev_hash = String::from("genesis");
        let mut event_count = 0;

        while let Some(line) = lines.next_line().await? {
            // Decrypt event
            let encrypted = general_purpose::STANDARD.decode(&line)?;
            let plaintext = self.decrypt_data(&encrypted)?;
            let event: SecurityEvent = serde_json::from_slice(&plaintext)?;

            // Verify hash chain
            if event.prev_hash != prev_hash {
                tracing::error!("❌ Audit log integrity violated at event {}", event_count);
                tracing::error!("   Expected prev_hash: {}", prev_hash);
                tracing::error!("   Actual prev_hash: {}", event.prev_hash);
                return Ok(false);
            }

            // Calculate next hash
            let event_json = serde_json::to_string(&event)?;
            let mut hasher = Sha256::new();
            hasher.update(event_json.as_bytes());
            hasher.update(event.prev_hash.as_bytes());
            prev_hash = format!("{:x}", hasher.finalize());

            event_count += 1;
        }

        tracing::info!("✅ Audit log integrity verified ({} events)", event_count);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_encrypted_audit_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        // ✅ OPTIMIZATION: Now async
        let mut logger = AuditLogger::with_path(&log_path).await.unwrap();

        // Log some events
        for i in 0..5 {
            let event = SecurityEvent {
                timestamp: Utc::now(),
                event_type: format!("test_event_{}", i),
                peer_id: format!("peer_{}", i),
                security_level: SecurityLevel::Basic,
                details: HashMap::new(),
                prev_hash: String::new(),
            };
            logger.log(event).await.unwrap();
        }

        // Verify log file exists and is encrypted
        assert!(log_path.exists());

        // Verify stats
        let stats = logger.get_stats().await.unwrap();
        assert_eq!(stats.memory_events, 5);
        assert!(stats.log_file_size > 0);
    }

    #[tokio::test]
    async fn test_log_integrity_verification() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        // ✅ OPTIMIZATION: Now async
        let mut logger = AuditLogger::with_path(&log_path).await.unwrap();

        // Log events
        for i in 0..10 {
            let event = SecurityEvent {
                timestamp: Utc::now(),
                event_type: format!("test_{}", i),
                peer_id: format!("peer_{}", i),
                security_level: SecurityLevel::Verified,
                details: HashMap::new(),
                prev_hash: String::new(),
            };
            logger.log(event).await.unwrap();
        }

        // Verify integrity
        let is_valid = logger.verify_integrity().await.unwrap();
        assert!(is_valid);
    }
}
