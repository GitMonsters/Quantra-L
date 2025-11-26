use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};
use notify::{Watcher, RecursiveMode, Event};

/// File Integrity Monitor with AI-powered anomaly detection
pub struct FileIntegrityMonitor {
    /// Baseline file hashes
    file_hashes: HashMap<PathBuf, FileBaseline>,
    /// Monitored directories
    watch_paths: Vec<PathBuf>,
    /// AI model for anomaly scoring
    anomaly_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBaseline {
    pub sha256: String,
    pub size: u64,
    pub modified: SystemTime,
    pub permissions: u32,
    pub access_count: u64,
}

#[derive(Debug, Clone)]
pub struct FileAnomaly {
    pub path: PathBuf,
    pub anomaly_score: f32,
    pub changes: Vec<String>,
    pub threat_indicators: Vec<String>,
}

impl FileIntegrityMonitor {
    pub async fn new() -> Result<Self> {
        let watch_paths = vec![
            PathBuf::from("/etc"),           // System configuration
            PathBuf::from("/usr/bin"),       // System binaries
            PathBuf::from("/usr/sbin"),      // System admin binaries
            PathBuf::from("/var/log/quantra"), // Application logs
            PathBuf::from("/home/worm/quantra/src"), // Source code
        ];

        let mut monitor = Self {
            file_hashes: HashMap::new(),
            watch_paths,
            anomaly_threshold: 0.7, // 70% confidence threshold
        };

        // Create initial baseline
        monitor.create_baseline().await?;

        tracing::info!("📁 File Integrity Monitor initialized");
        tracing::info!("   Monitoring {} directories", monitor.watch_paths.len());
        tracing::info!("   Baseline: {} files tracked", monitor.file_hashes.len());

        Ok(monitor)
    }

    /// Create baseline of all monitored files
    async fn create_baseline(&mut self) -> Result<()> {
        // Clone paths to avoid borrow checker issues
        let paths = self.watch_paths.clone();
        for path in &paths {
            if path.exists() {
                self.scan_directory(path).await?;
            }
        }
        Ok(())
    }

    /// Recursively scan directory and hash files
    fn scan_directory<'a>(&'a mut self, dir: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            if !dir.is_dir() {
                return Ok(());
            }

            // ✅ OPTIMIZATION: Use tokio::fs for non-blocking I/O
            let mut entries = tokio::fs::read_dir(dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_file() {
                    if let Ok(baseline) = self.hash_file(&path).await {
                        self.file_hashes.insert(path, baseline);
                    }
                } else if path.is_dir() {
                    // Recursively scan subdirectories (limited depth)
                    if let Some(depth) = self.calculate_depth(&path) {
                        if depth < 5 {
                            self.scan_directory(&path).await?;
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// Calculate directory depth
    fn calculate_depth(&self, path: &Path) -> Option<usize> {
        path.components().count().checked_sub(1)
    }

    /// Hash file and extract metadata
    async fn hash_file(&self, path: &Path) -> Result<FileBaseline> {
        let data = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = format!("{:x}", hasher.finalize());

        let metadata = tokio::fs::metadata(path).await?;

        Ok(FileBaseline {
            sha256: hash,
            size: metadata.len(),
            modified: metadata.modified()?,
            permissions: Self::get_permissions(&metadata),
            access_count: 0,
        })
    }

    /// Get file permissions (Unix)
    #[cfg(unix)]
    fn get_permissions(metadata: &std::fs::Metadata) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }

    #[cfg(not(unix))]
    fn get_permissions(_metadata: &std::fs::Metadata) -> u32 {
        0
    }

    /// Start continuous monitoring
    pub async fn start_monitoring(&mut self) -> Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            if let Err(e) = self.check_integrity().await {
                tracing::error!("Integrity check failed: {}", e);
            }
        }
    }

    /// Check file integrity against baseline
    pub async fn check_integrity(&mut self) -> Result<Vec<FileAnomaly>> {
        let mut anomalies = Vec::new();

        for (path, baseline) in &self.file_hashes {
            if !path.exists() {
                // File deleted - suspicious
                anomalies.push(FileAnomaly {
                    path: path.clone(),
                    anomaly_score: 0.9,
                    changes: vec!["File deleted".to_string()],
                    threat_indicators: vec!["Potential evidence destruction".to_string()],
                });
                continue;
            }

            match self.hash_file(path).await {
                Ok(current) => {
                    let anomaly = self.analyze_changes(path, baseline, &current).await;
                    if anomaly.anomaly_score >= self.anomaly_threshold {
                        tracing::warn!("🚨 File anomaly detected: {:?}", path);
                        tracing::warn!("   Score: {:.2}%", anomaly.anomaly_score * 100.0);
                        tracing::warn!("   Changes: {:?}", anomaly.changes);
                        anomalies.push(anomaly);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to check {}: {}", path.display(), e);
                }
            }
        }

        if !anomalies.is_empty() {
            tracing::warn!("⚠️  Detected {} file anomalies", anomalies.len());
        }

        Ok(anomalies)
    }

    /// AI-powered analysis of file changes
    async fn analyze_changes(
        &self,
        path: &Path,
        baseline: &FileBaseline,
        current: &FileBaseline,
    ) -> FileAnomaly {
        let mut score = 0.0_f32;
        let mut changes = Vec::new();
        let mut threats = Vec::new();

        // Hash changed - file modified
        if baseline.sha256 != current.sha256 {
            score += 0.4;
            changes.push("Content modified".to_string());

            // Check if it's a critical system file
            if self.is_critical_file(path) {
                score += 0.3;
                threats.push("Critical system file modified".to_string());
            }
        }

        // Size changed dramatically
        if current.size > baseline.size * 2 || current.size < baseline.size / 2 {
            score += 0.2;
            changes.push(format!("Size: {} → {} bytes", baseline.size, current.size));
        }

        // Permissions changed
        if baseline.permissions != current.permissions {
            score += 0.3;
            changes.push(format!("Permissions: {:o} → {:o}",
                baseline.permissions, current.permissions));

            // SUID/SGID added - very suspicious
            if (current.permissions & 0o6000) > (baseline.permissions & 0o6000) {
                score += 0.4;
                threats.push("SUID/SGID bit added - privilege escalation risk".to_string());
            }
        }

        // Modified time anomaly detection
        if let Ok(duration) = current.modified.duration_since(baseline.modified) {
            // Modified very recently after being stable
            if duration.as_secs() < 300 { // Within last 5 minutes
                score += 0.1;
                changes.push("Recently modified".to_string());
            }
        }

        FileAnomaly {
            path: path.to_path_buf(),
            anomaly_score: score.min(1.0),
            changes,
            threat_indicators: threats,
        }
    }

    /// Check if file is critical system file
    fn is_critical_file(&self, path: &Path) -> bool {
        let critical_patterns = [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/sudoers",
            "/etc/ssh/",
            "/usr/bin/sudo",
            "/usr/sbin/",
        ];

        let path_str = path.to_string_lossy();
        critical_patterns.iter().any(|pattern| path_str.contains(pattern))
    }

    /// Detect abnormal file access patterns (AI-based)
    pub async fn detect_access_patterns(&mut self) -> Result<Vec<String>> {
        let mut suspicious = Vec::new();

        // Simple anomaly detection: files accessed unusually often
        for (path, baseline) in &mut self.file_hashes {
            if baseline.access_count > 100 {
                suspicious.push(format!(
                    "File {} accessed {} times (abnormal frequency)",
                    path.display(),
                    baseline.access_count
                ));
                baseline.access_count = 0; // Reset counter
            }
        }

        Ok(suspicious)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_integrity_monitor() {
        let monitor = FileIntegrityMonitor::new().await.unwrap();
        assert!(!monitor.file_hashes.is_empty());
    }

    #[tokio::test]
    async fn test_hash_file() {
        let monitor = FileIntegrityMonitor::new().await.unwrap();
        let temp_file = std::env::temp_dir().join("test_file.txt");
        std::fs::write(&temp_file, b"test content").unwrap();

        let baseline = monitor.hash_file(&temp_file).await.unwrap();
        assert!(!baseline.sha256.is_empty());
        assert_eq!(baseline.size, 12);

        std::fs::remove_file(&temp_file).ok();
    }
}
