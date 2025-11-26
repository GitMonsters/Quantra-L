use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc, Duration, Timelike};
use serde::{Serialize, Deserialize};
use crate::security::SecurityEvent;

/// Behavioral analyzer using pattern recognition
pub struct BehavioralAnalyzer {
    /// User behavior profiles
    profiles: HashMap<String, UserProfile>,
    /// Recent events for pattern matching
    event_buffer: VecDeque<SecurityEvent>,
    /// Learned patterns
    patterns: Vec<BehaviorPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserProfile {
    user_id: String,
    typical_hours: Vec<u32>, // Hours when user is typically active
    typical_actions: HashMap<String, f64>, // Action frequency
    anomaly_score: f64,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BehaviorPattern {
    name: String,
    description: String,
    events: Vec<String>,
    time_window_secs: i64,
    confidence: f64,
}

impl BehavioralAnalyzer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            profiles: HashMap::new(),
            event_buffer: VecDeque::with_capacity(1000),
            patterns: Self::load_attack_patterns(),
        })
    }

    /// Load known attack patterns
    fn load_attack_patterns() -> Vec<BehaviorPattern> {
        vec![
            BehaviorPattern {
                name: "Data Exfiltration".to_string(),
                description: "Unusual file access followed by network activity".to_string(),
                events: vec![
                    "FileAccess".to_string(),
                    "NetworkSuspicious".to_string(),
                ],
                time_window_secs: 300, // 5 minutes
                confidence: 0.8,
            },
            BehaviorPattern {
                name: "Privilege Escalation".to_string(),
                description: "Permission changes followed by system file access".to_string(),
                events: vec![
                    "PermissionChange".to_string(),
                    "SystemFileAccess".to_string(),
                ],
                time_window_secs: 180,
                confidence: 0.9,
            },
            BehaviorPattern {
                name: "Evidence Destruction".to_string(),
                description: "Multiple file deletions in quick succession".to_string(),
                events: vec![
                    "FileDeleted".to_string(),
                    "FileDeleted".to_string(),
                    "FileDeleted".to_string(),
                ],
                time_window_secs: 60,
                confidence: 0.85,
            },
            BehaviorPattern {
                name: "Lateral Movement".to_string(),
                description: "SSH connections to multiple internal hosts".to_string(),
                events: vec![
                    "SSHConnection".to_string(),
                    "SSHConnection".to_string(),
                ],
                time_window_secs: 600,
                confidence: 0.7,
            },
        ]
    }

    /// Start continuous behavioral analysis
    pub async fn start_analysis(&mut self) -> Result<()> {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            // Analyze for attack patterns
            if let Some(pattern) = self.detect_attack_pattern().await? {
                tracing::warn!("🎯 Attack pattern detected: {}", pattern.name);
                tracing::warn!("   Description: {}", pattern.description);
                tracing::warn!("   Confidence: {:.1}%", pattern.confidence * 100.0);
            }

            // Clean old events
            self.cleanup_old_events();
        }
    }

    /// Record security event for behavioral analysis
    pub async fn record_event(&mut self, event: &SecurityEvent) -> Result<()> {
        self.event_buffer.push_back(event.clone());

        // Update user profile
        self.update_user_profile(&event.source, event).await?;

        Ok(())
    }

    /// Update user behavior profile
    async fn update_user_profile(&mut self, user_id: &str, event: &SecurityEvent) -> Result<()> {
        // First, update the profile data
        {
            let profile = self.profiles.entry(user_id.to_string())
                .or_insert_with(|| UserProfile {
                    user_id: user_id.to_string(),
                    typical_hours: Vec::new(),
                    typical_actions: HashMap::new(),
                    anomaly_score: 0.0,
                    first_seen: event.timestamp,
                    last_seen: event.timestamp,
                });

            // Update last seen
            profile.last_seen = event.timestamp;

            // Track typical hours
            let hour = event.timestamp.hour();
            if !profile.typical_hours.contains(&hour) {
                profile.typical_hours.push(hour);
            }

            // Track action frequencies
            let action_key = format!("{:?}", event.event_type);
            profile.typical_actions
                .entry(action_key)
                .and_modify(|count| *count += 1.0)
                .or_insert(1.0);
        }

        // Calculate anomaly score after mutable borrow ends
        if let Some(profile) = self.profiles.get(user_id) {
            let anomaly_score = self.calculate_user_anomaly(profile, event);

            if let Some(profile) = self.profiles.get_mut(user_id) {
                profile.anomaly_score = anomaly_score;

                if profile.anomaly_score > 0.7 {
                    tracing::warn!("👤 User {} has high anomaly score: {:.2}",
                        user_id, profile.anomaly_score);
                }
            }
        }

        Ok(())
    }

    /// Calculate anomaly score for user behavior
    fn calculate_user_anomaly(&self, profile: &UserProfile, event: &SecurityEvent) -> f64 {
        let mut score: f64 = 0.0;

        // Check if activity is at unusual hour
        let hour = event.timestamp.hour();
        if !profile.typical_hours.contains(&hour) {
            score += 0.3;
        }

        // Check if action is unusual for this user
        let action_key = format!("{:?}", event.event_type);
        if !profile.typical_actions.contains_key(&action_key) {
            score += 0.4;
        }

        // Check if action frequency is abnormal
        if let Some(&freq) = profile.typical_actions.get(&action_key) {
            // Count recent similar actions
            let recent_count = self.event_buffer.iter()
                .filter(|e| e.source == profile.user_id)
                .filter(|e| format!("{:?}", e.event_type) == action_key)
                .filter(|e| (Utc::now() - e.timestamp).num_hours() < 1)
                .count() as f64;

            // If current rate is 3x higher than typical, flag it
            if recent_count > freq * 3.0 {
                score += 0.3;
            }
        }

        score.min(1.0)
    }

    /// Detect attack patterns in event buffer
    async fn detect_attack_pattern(&self) -> Result<Option<BehaviorPattern>> {
        for pattern in &self.patterns {
            if self.matches_pattern(pattern) {
                return Ok(Some(pattern.clone()));
            }
        }
        Ok(None)
    }

    /// Check if recent events match attack pattern
    fn matches_pattern(&self, pattern: &BehaviorPattern) -> bool {
        let cutoff = Utc::now() - Duration::seconds(pattern.time_window_secs);

        // Get recent events within time window
        let recent_events: Vec<&SecurityEvent> = self.event_buffer.iter()
            .filter(|e| e.timestamp > cutoff)
            .collect();

        // Check if pattern events occur in sequence
        let mut pattern_index = 0;

        for event in recent_events {
            let event_type = format!("{:?}", event.event_type);

            if pattern_index < pattern.events.len() &&
               (pattern.events[pattern_index] == event_type ||
                pattern.events[pattern_index].contains(&event_type)) {
                pattern_index += 1;

                if pattern_index == pattern.events.len() {
                    return true; // All pattern events matched
                }
            }
        }

        false
    }

    /// Clean old events from buffer
    fn cleanup_old_events(&mut self) {
        let cutoff = Utc::now() - Duration::hours(24);

        while let Some(event) = self.event_buffer.front() {
            if event.timestamp < cutoff {
                self.event_buffer.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get user risk assessment
    pub async fn get_user_risk(&self, user_id: &str) -> Option<f64> {
        self.profiles.get(user_id).map(|p| p.anomaly_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::EventType;

    #[tokio::test]
    async fn test_behavioral_analyzer() {
        let mut analyzer = BehavioralAnalyzer::new().unwrap();

        let event = SecurityEvent {
            event_type: EventType::FileModified,
            timestamp: Utc::now(),
            source: "test_user".to_string(),
            details: serde_json::json!({}),
        };

        analyzer.record_event(&event).await.unwrap();

        let risk = analyzer.get_user_risk("test_user").await;
        assert!(risk.is_some());
    }

    #[test]
    fn test_attack_patterns_loaded() {
        let analyzer = BehavioralAnalyzer::new().unwrap();
        assert!(!analyzer.patterns.is_empty());
        assert!(analyzer.patterns.len() >= 4);
    }
}
