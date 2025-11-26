pub mod monitor;
pub mod anomaly;
pub mod emergency;
pub mod behavioral;
pub mod mirror_shield;
pub mod bait_wallet;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Security monitoring orchestrator
pub struct SecurityMonitor {
    pub file_monitor: Arc<RwLock<monitor::FileIntegrityMonitor>>,
    pub anomaly_detector: Arc<RwLock<anomaly::AnomalyDetector>>,
    pub emergency_handler: Arc<RwLock<emergency::EmergencyHandler>>,
    pub behavioral_analyzer: Arc<RwLock<behavioral::BehavioralAnalyzer>>,
    pub mirror_shield: Arc<RwLock<mirror_shield::MirrorShield>>,
    pub bait_manager: Arc<RwLock<bait_wallet::BaitWalletManager>>,
}

impl SecurityMonitor {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            file_monitor: Arc::new(RwLock::new(monitor::FileIntegrityMonitor::new().await?)),
            anomaly_detector: Arc::new(RwLock::new(anomaly::AnomalyDetector::new()?)),
            emergency_handler: Arc::new(RwLock::new(emergency::EmergencyHandler::new()?)),
            behavioral_analyzer: Arc::new(RwLock::new(behavioral::BehavioralAnalyzer::new()?)),
            mirror_shield: Arc::new(RwLock::new(mirror_shield::MirrorShield::new())),
            bait_manager: Arc::new(RwLock::new(bait_wallet::BaitWalletManager::new("https://callback.quantra.local"))),
        })
    }

    /// Start all monitoring services
    pub async fn start(&self) -> Result<()> {
        tracing::info!("🤖 Starting AI Security Monitoring System");

        // Start file integrity monitoring
        let file_monitor = self.file_monitor.clone();
        tokio::spawn(async move {
            if let Err(e) = file_monitor.write().await.start_monitoring().await {
                tracing::error!("File monitor error: {}", e);
            }
        });

        // Start anomaly detection
        let anomaly_detector = self.anomaly_detector.clone();
        tokio::spawn(async move {
            if let Err(e) = anomaly_detector.write().await.start_analysis().await {
                tracing::error!("Anomaly detector error: {}", e);
            }
        });

        // Start behavioral analysis
        let behavioral_analyzer = self.behavioral_analyzer.clone();
        tokio::spawn(async move {
            if let Err(e) = behavioral_analyzer.write().await.start_analysis().await {
                tracing::error!("Behavioral analyzer error: {}", e);
            }
        });

        tracing::info!("✅ AI Security Monitoring System started");
        Ok(())
    }

    /// Report security event for analysis
    pub async fn report_event(&self, event: SecurityEvent) -> Result<()> {
        // Analyze with AI
        let threat_level = self.anomaly_detector.write().await.analyze_event(&event).await?;

        if threat_level >= ThreatLevel::High {
            tracing::warn!("🚨 High threat detected: {:?}", event);

            // Trigger emergency response if critical
            if threat_level == ThreatLevel::Critical {
                self.emergency_handler.write().await.handle_critical_threat(&event).await?;
            }
        }

        // Record for behavioral analysis
        self.behavioral_analyzer.write().await.record_event(&event).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub event_type: EventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    FileModified,
    UnauthorizedAccess,
    PowerAnomaly,
    NetworkSuspicious,
    ProcessAnomalous,
    HardwareEvent,
    SoftwareUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}
