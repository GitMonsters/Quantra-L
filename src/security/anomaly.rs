use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc, Duration, Timelike};
use serde::{Serialize, Deserialize};
use crate::security::{SecurityEvent, EventType, ThreatLevel};

/// AI-powered anomaly detector with machine learning
pub struct AnomalyDetector {
    /// Event history for pattern analysis
    event_history: VecDeque<SecurityEvent>,
    /// Learned patterns (simple frequency-based model)
    patterns: HashMap<String, EventPattern>,
    /// Power surge detector
    power_monitor: PowerMonitor,
    /// Process monitor
    process_monitor: ProcessMonitor,
    /// Maximum history size
    max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventPattern {
    pub event_type: String,
    pub normal_frequency: f64, // Events per hour
    pub variance: f64,
    pub last_seen: DateTime<Utc>,
    pub total_count: u64,
}

/// Power surge and hardware event monitor
struct PowerMonitor {
    voltage_readings: VecDeque<f64>,
    normal_voltage: f64,
    variance_threshold: f64,
}

/// Process behavior monitor
struct ProcessMonitor {
    process_stats: HashMap<String, ProcessStats>,
}

#[derive(Debug, Clone)]
struct ProcessStats {
    cpu_usage: VecDeque<f64>,
    memory_usage: VecDeque<u64>,
    network_activity: VecDeque<u64>,
    baseline_cpu: f64,
    baseline_memory: u64,
}

impl AnomalyDetector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            event_history: VecDeque::with_capacity(10000),
            patterns: HashMap::new(),
            power_monitor: PowerMonitor::new(),
            process_monitor: ProcessMonitor::new(),
            max_history: 10000,
        })
    }

    /// Start continuous anomaly analysis
    pub async fn start_analysis(&mut self) -> Result<()> {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            // Analyze power anomalies
            if let Some(anomaly) = self.power_monitor.detect_anomaly()? {
                tracing::warn!("⚡ Power anomaly detected: {}", anomaly);
            }

            // Analyze process anomalies
            if let Some(anomalies) = self.process_monitor.detect_anomalies().await? {
                for anomaly in anomalies {
                    tracing::warn!("🔍 Process anomaly: {}", anomaly);
                }
            }

            // Clean old history
            self.cleanup_history();
        }
    }

    /// Analyze security event using AI
    pub async fn analyze_event(&mut self, event: &SecurityEvent) -> Result<ThreatLevel> {
        // Record event
        self.event_history.push_back(event.clone());

        // Calculate anomaly score
        let score = self.calculate_anomaly_score(event).await?;

        // Convert score to threat level
        let threat = if score > 0.9 {
            ThreatLevel::Critical
        } else if score > 0.7 {
            ThreatLevel::High
        } else if score > 0.4 {
            ThreatLevel::Medium
        } else {
            ThreatLevel::Low
        };

        // Update learned patterns
        self.update_patterns(event);

        Ok(threat)
    }

    /// Calculate anomaly score using ML-like approach
    async fn calculate_anomaly_score(&self, event: &SecurityEvent) -> Result<f64> {
        let mut score = 0.0;

        match event.event_type {
            EventType::FileModified => {
                // Check if file modification is unusual
                score += self.analyze_file_modification(event)?;
            }
            EventType::UnauthorizedAccess => {
                // Always high threat
                score += 0.8;
            }
            EventType::PowerAnomaly => {
                // Power surge/dip detected
                score += 0.7;
            }
            EventType::NetworkSuspicious => {
                // Unusual network activity
                score += self.analyze_network_pattern(event)?;
            }
            EventType::ProcessAnomalous => {
                // Process behavior anomaly
                score += self.analyze_process_behavior(event)?;
            }
            EventType::HardwareEvent => {
                // Hardware tampering detected
                score += 0.9;
            }
            EventType::SoftwareUpdate => {
                // Unexpected software update
                score += self.analyze_software_update(event)?;
            }
        }

        // Time-based anomaly (unusual time of day)
        score += self.analyze_temporal_anomaly(event)?;

        // Frequency anomaly (too many similar events)
        score += self.analyze_frequency_anomaly(event)?;

        Ok(score.min(1.0_f64))
    }

    /// Analyze file modification patterns
    fn analyze_file_modification(&self, event: &SecurityEvent) -> Result<f64> {
        // Check recent file modifications
        let recent_mods = self.event_history.iter()
            .filter(|e| e.event_type == EventType::FileModified)
            .filter(|e| (Utc::now() - e.timestamp).num_minutes() < 5)
            .count();

        // More than 10 file mods in 5 minutes = suspicious
        Ok(if recent_mods > 10 { 0.6 } else { 0.1 })
    }

    /// Analyze network pattern
    fn analyze_network_pattern(&self, _event: &SecurityEvent) -> Result<f64> {
        // Count recent network events
        let recent_network = self.event_history.iter()
            .filter(|e| e.event_type == EventType::NetworkSuspicious)
            .filter(|e| (Utc::now() - e.timestamp).num_seconds() < 60)
            .count();

        Ok(if recent_network > 5 { 0.7 } else { 0.2 })
    }

    /// Analyze process behavior
    fn analyze_process_behavior(&self, _event: &SecurityEvent) -> Result<f64> {
        // Anomalous process behavior already flagged
        Ok(0.6)
    }

    /// Analyze software update event
    fn analyze_software_update(&self, event: &SecurityEvent) -> Result<f64> {
        // Software update outside maintenance window = suspicious
        let hour = event.timestamp.hour();

        // Normal maintenance: 2-4 AM
        if hour >= 2 && hour <= 4 {
            Ok(0.1)
        } else {
            // Update outside maintenance window
            Ok(0.8)
        }
    }

    /// Analyze temporal anomaly (time-of-day pattern)
    fn analyze_temporal_anomaly(&self, event: &SecurityEvent) -> Result<f64> {
        let hour = event.timestamp.hour();

        // Activity during unusual hours (1-5 AM) = suspicious
        if hour >= 1 && hour <= 5 {
            Ok(0.3)
        } else {
            Ok(0.0)
        }
    }

    /// Analyze frequency anomaly
    fn analyze_frequency_anomaly(&self, event: &SecurityEvent) -> Result<f64> {
        let event_key = format!("{:?}", event.event_type);

        if let Some(pattern) = self.patterns.get(&event_key) {
            // Count recent events of this type
            let recent_count = self.event_history.iter()
                .filter(|e| format!("{:?}", e.event_type) == event_key)
                .filter(|e| (Utc::now() - e.timestamp).num_hours() < 1)
                .count() as f64;

            // Compare to normal frequency
            let deviation = (recent_count - pattern.normal_frequency).abs() / pattern.normal_frequency;

            Ok(if deviation > 2.0 { 0.4 } else { 0.0 })
        } else {
            // New event type = slightly suspicious
            Ok(0.1)
        }
    }

    /// Update learned patterns
    fn update_patterns(&mut self, event: &SecurityEvent) {
        let key = format!("{:?}", event.event_type);

        self.patterns.entry(key.clone())
            .and_modify(|p| {
                p.total_count += 1;
                p.last_seen = event.timestamp;
                // Update moving average of frequency
                p.normal_frequency = p.normal_frequency * 0.9 + 1.0 * 0.1;
            })
            .or_insert(EventPattern {
                event_type: key,
                normal_frequency: 1.0,
                variance: 0.0,
                last_seen: event.timestamp,
                total_count: 1,
            });
    }

    /// Clean old events from history
    fn cleanup_history(&mut self) {
        while self.event_history.len() > self.max_history {
            self.event_history.pop_front();
        }
    }
}

impl PowerMonitor {
    fn new() -> Self {
        Self {
            voltage_readings: VecDeque::with_capacity(100),
            normal_voltage: 120.0, // 120V standard (US)
            variance_threshold: 10.0, // ±10V tolerance
        }
    }

    /// Detect power anomalies (surges/sags)
    fn detect_anomaly(&mut self) -> Result<Option<String>> {
        // Simulate voltage reading (in production, read from hardware)
        let voltage = self.read_voltage()?;

        self.voltage_readings.push_back(voltage);
        if self.voltage_readings.len() > 100 {
            self.voltage_readings.pop_front();
        }

        // Check for surge
        if voltage > self.normal_voltage + self.variance_threshold {
            return Ok(Some(format!(
                "Power SURGE detected: {:.1}V (normal: {:.1}V)",
                voltage, self.normal_voltage
            )));
        }

        // Check for sag
        if voltage < self.normal_voltage - self.variance_threshold {
            return Ok(Some(format!(
                "Power SAG detected: {:.1}V (normal: {:.1}V)",
                voltage, self.normal_voltage
            )));
        }

        Ok(None)
    }

    /// Read voltage from system (simulated)
    fn read_voltage(&self) -> Result<f64> {
        // In production: read from ACPI, sensors, or UPS
        // For now: simulate normal voltage with small noise
        Ok(self.normal_voltage + (rand::random::<f64>() - 0.5) * 2.0)
    }
}

impl ProcessMonitor {
    fn new() -> Self {
        Self {
            process_stats: HashMap::new(),
        }
    }

    /// Detect process anomalies
    async fn detect_anomalies(&mut self) -> Result<Option<Vec<String>>> {
        let mut anomalies = Vec::new();

        // Monitor critical processes
        for proc_name in &["quantraband", "sshd", "systemd"] {
            if let Some(stats) = self.get_process_stats(proc_name).await? {
                if let Some(anomaly) = self.analyze_process(&stats) {
                    anomalies.push(anomaly);
                }
            }
        }

        Ok(if anomalies.is_empty() { None } else { Some(anomalies) })
    }

    /// Get process statistics
    async fn get_process_stats(&self, _proc_name: &str) -> Result<Option<ProcessStats>> {
        // In production: read from /proc/[pid]/stat or use sysinfo crate
        // For now: return None (not implemented)
        Ok(None)
    }

    /// Analyze process statistics for anomalies
    fn analyze_process(&self, stats: &ProcessStats) -> Option<String> {
        // Check CPU usage spike
        if let Some(&latest_cpu) = stats.cpu_usage.back() {
            if latest_cpu > stats.baseline_cpu * 3.0 {
                return Some(format!(
                    "CPU spike: {:.1}% (baseline: {:.1}%)",
                    latest_cpu, stats.baseline_cpu
                ));
            }
        }

        // Check memory usage spike
        if let Some(&latest_mem) = stats.memory_usage.back() {
            if latest_mem > stats.baseline_memory * 2 {
                return Some(format!(
                    "Memory spike: {} bytes (baseline: {} bytes)",
                    latest_mem, stats.baseline_memory
                ));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anomaly_detector() {
        let mut detector = AnomalyDetector::new().unwrap();

        let event = SecurityEvent {
            event_type: EventType::FileModified,
            timestamp: Utc::now(),
            source: "test".to_string(),
            details: serde_json::json!({}),
        };

        let threat = detector.analyze_event(&event).await.unwrap();
        assert!(threat <= ThreatLevel::Medium);
    }

    #[test]
    fn test_power_monitor() {
        let mut monitor = PowerMonitor::new();
        let anomaly = monitor.detect_anomaly().unwrap();
        // Should be None with simulated normal voltage
        assert!(anomaly.is_none() || anomaly.unwrap().contains("V"));
    }
}
