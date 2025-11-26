//! Bait Wallet System - Crypto Honeypot with Location Tracking
//! Deploys fake crypto wallets that phone home when accessed

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Bait wallet types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WalletType {
    Bitcoin,
    Ethereum,
    Monero,
    Solana,
    Generic,
}

/// Fake wallet seed phrases (HONEYPOT - DO NOT USE)
const BAIT_SEEDS: &[&str] = &[
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    "legal winner thank year wave sausage worth useful legal winner thank yellow",
    "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
    "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
];

/// Tracked access event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaitAccessEvent {
    pub timestamp: DateTime<Utc>,
    pub wallet_id: String,
    pub wallet_type: WalletType,
    pub attacker_ip: String,
    pub attacker_location: Option<GeoLocation>,
    pub user_agent: Option<String>,
    pub access_type: AccessType,
    pub transaction_attempted: bool,
    pub alert_sent: bool,
}

/// Geolocation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub ip: String,
    pub country: String,
    pub country_code: String,
    pub region: String,
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
    pub isp: String,
    pub org: String,
    pub timezone: String,
    pub is_vpn: bool,
    pub is_tor: bool,
    pub is_proxy: bool,
}

/// Type of access to bait wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessType {
    /// Checked balance
    BalanceCheck,
    /// Imported wallet
    WalletImport,
    /// Attempted transaction
    TransactionAttempt,
    /// Private key export attempt
    KeyExport,
    /// API access
    ApiAccess,
}

/// Bait wallet definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaitWallet {
    pub id: String,
    pub wallet_type: WalletType,
    pub address: String,
    pub seed_phrase: String,
    pub private_key: String,
    pub fake_balance: String,
    pub created_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: Option<DateTime<Utc>>,
    pub callback_url: String,
    pub active: bool,
}

/// Bait Wallet Manager
pub struct BaitWalletManager {
    /// Active bait wallets
    wallets: Arc<RwLock<HashMap<String, BaitWallet>>>,
    /// Access events
    access_log: Arc<RwLock<Vec<BaitAccessEvent>>>,
    /// Callback URL for alerts
    callback_url: String,
    /// Alert webhook
    alert_webhook: Option<String>,
}

impl BaitWalletManager {
    /// Create new bait wallet manager
    pub fn new(callback_url: &str) -> Self {
        tracing::info!("🎣 Bait Wallet System ACTIVATED");
        tracing::info!("   Callback: {}", callback_url);

        Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            access_log: Arc::new(RwLock::new(Vec::new())),
            callback_url: callback_url.to_string(),
            alert_webhook: None,
        }
    }

    /// Set alert webhook (Slack, Discord, etc.)
    pub fn set_alert_webhook(&mut self, webhook: &str) {
        self.alert_webhook = Some(webhook.to_string());
        tracing::info!("🔔 Alert webhook configured");
    }

    /// Deploy a new bait wallet
    pub async fn deploy_bait(&self, wallet_type: WalletType, fake_balance: &str) -> Result<BaitWallet> {
        let id = uuid::Uuid::new_v4().to_string();
        let seed_idx = rand::random::<usize>() % BAIT_SEEDS.len();

        let wallet = BaitWallet {
            id: id.clone(),
            wallet_type: wallet_type.clone(),
            address: self.generate_fake_address(&wallet_type),
            seed_phrase: BAIT_SEEDS[seed_idx].to_string(),
            private_key: self.generate_fake_private_key(&wallet_type),
            fake_balance: fake_balance.to_string(),
            created_at: Utc::now(),
            access_count: 0,
            last_accessed: None,
            callback_url: format!("{}/bait/{}", self.callback_url, id),
            active: true,
        };

        self.wallets.write().await.insert(id.clone(), wallet.clone());

        tracing::warn!(
            "🎣 BAIT DEPLOYED: {:?} wallet with {} (fake)",
            wallet_type,
            fake_balance
        );
        tracing::warn!("   Address: {}", wallet.address);
        tracing::warn!("   Callback: {}", wallet.callback_url);

        Ok(wallet)
    }

    /// Deploy multiple bait wallets (honeypot cluster)
    pub async fn deploy_honeypot_cluster(&self) -> Result<Vec<BaitWallet>> {
        let mut wallets = Vec::new();

        // Deploy various wallet types with enticing balances
        wallets.push(self.deploy_bait(WalletType::Bitcoin, "2.5 BTC").await?);
        wallets.push(self.deploy_bait(WalletType::Ethereum, "15.7 ETH").await?);
        wallets.push(self.deploy_bait(WalletType::Solana, "500 SOL").await?);
        wallets.push(self.deploy_bait(WalletType::Monero, "50 XMR").await?);

        tracing::warn!("🍯 HONEYPOT CLUSTER DEPLOYED: {} wallets active", wallets.len());

        Ok(wallets)
    }

    /// Handle bait wallet access (CALL HOME)
    pub async fn handle_access(
        &self,
        wallet_id: &str,
        attacker_ip: &str,
        access_type: AccessType,
        user_agent: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();

        // Get wallet
        let mut wallets = self.wallets.write().await;
        let wallet = match wallets.get_mut(wallet_id) {
            Some(w) => w,
            None => return Ok(()), // Unknown wallet ID
        };

        wallet.access_count += 1;
        wallet.last_accessed = Some(now);
        let wallet_type = wallet.wallet_type.clone();
        let wallet_address = wallet.address.clone();
        drop(wallets);

        // Get attacker location
        let location = self.get_geolocation(attacker_ip).await?;

        // Log the event
        let event = BaitAccessEvent {
            timestamp: now,
            wallet_id: wallet_id.to_string(),
            wallet_type: wallet_type.clone(),
            attacker_ip: attacker_ip.to_string(),
            attacker_location: location.clone(),
            user_agent: user_agent.map(String::from),
            access_type: access_type.clone(),
            transaction_attempted: matches!(access_type, AccessType::TransactionAttempt),
            alert_sent: true,
        };

        self.access_log.write().await.push(event.clone());

        // ALERT!
        self.send_alert(&event, &wallet_address).await?;

        // Log to console
        tracing::error!("🚨 BAIT WALLET ACCESSED!");
        tracing::error!("   Wallet: {:?} ({})", wallet_type, wallet_id);
        tracing::error!("   Address: {}", wallet_address);
        tracing::error!("   Attacker IP: {}", attacker_ip);
        tracing::error!("   Access Type: {:?}", access_type);

        if let Some(loc) = &location {
            tracing::error!("   📍 LOCATION: {}, {}, {}", loc.city, loc.region, loc.country);
            tracing::error!("   📍 Coordinates: {:.4}, {:.4}", loc.latitude, loc.longitude);
            tracing::error!("   📍 ISP: {}", loc.isp);
            if loc.is_vpn {
                tracing::error!("   ⚠️ VPN DETECTED");
            }
            if loc.is_tor {
                tracing::error!("   ⚠️ TOR EXIT NODE");
            }
        }

        Ok(())
    }

    /// Get geolocation for IP
    async fn get_geolocation(&self, ip: &str) -> Result<Option<GeoLocation>> {
        // In production, use real geolocation API (ip-api.com, ipinfo.io, etc.)
        // For now, return mock data for testing

        // Detect common VPN/Tor ranges
        let is_vpn = ip.starts_with("10.") || ip.starts_with("192.168.") || ip.starts_with("172.");
        let is_tor = ip.contains("tor") || ip.ends_with(".onion");

        Ok(Some(GeoLocation {
            ip: ip.to_string(),
            country: "Unknown".to_string(),
            country_code: "XX".to_string(),
            region: "Unknown".to_string(),
            city: "Unknown".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            isp: "Unknown ISP".to_string(),
            org: "Unknown Org".to_string(),
            timezone: "UTC".to_string(),
            is_vpn,
            is_tor,
            is_proxy: false,
        }))
    }

    /// Send alert when bait is accessed
    async fn send_alert(&self, event: &BaitAccessEvent, address: &str) -> Result<()> {
        let alert_msg = format!(
            "🚨 BAIT WALLET ALERT!\n\
             Wallet: {:?}\n\
             Address: {}\n\
             Attacker IP: {}\n\
             Access Type: {:?}\n\
             Location: {}\n\
             Time: {}",
            event.wallet_type,
            address,
            event.attacker_ip,
            event.access_type,
            event.attacker_location.as_ref()
                .map(|l| format!("{}, {}", l.city, l.country))
                .unwrap_or_else(|| "Unknown".to_string()),
            event.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        tracing::warn!("{}", alert_msg);

        // Send to webhook if configured
        if let Some(webhook) = &self.alert_webhook {
            // In production: HTTP POST to webhook
            tracing::info!("📡 Alert sent to webhook: {}", webhook);
        }

        Ok(())
    }

    /// Generate fake wallet address
    fn generate_fake_address(&self, wallet_type: &WalletType) -> String {
        let mut hasher = Sha256::new();
        hasher.update(uuid::Uuid::new_v4().to_string().as_bytes());
        let hash1 = format!("{:x}", hasher.finalize());

        // Generate second hash for longer addresses
        let mut hasher2 = Sha256::new();
        hasher2.update(uuid::Uuid::new_v4().to_string().as_bytes());
        let hash2 = format!("{:x}", hasher2.finalize());
        let combined = format!("{}{}", hash1, hash2);

        match wallet_type {
            WalletType::Bitcoin => format!("bc1q{}", &hash1[..38]),
            WalletType::Ethereum => format!("0x{}", &hash1[..40]),
            WalletType::Solana => hash1[..44].to_string(),
            WalletType::Monero => format!("4{}", &combined[..94]),
            WalletType::Generic => hash1[..42].to_string(),
        }
    }

    /// Generate fake private key
    fn generate_fake_private_key(&self, wallet_type: &WalletType) -> String {
        let mut hasher = Sha256::new();
        hasher.update(uuid::Uuid::new_v4().to_string().as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        match wallet_type {
            WalletType::Bitcoin => format!("5{}", &hash[..50]),
            WalletType::Ethereum => format!("0x{}", &hash),
            _ => hash,
        }
    }

    /// Get all bait wallets
    pub async fn get_all_wallets(&self) -> Vec<BaitWallet> {
        self.wallets.read().await.values().cloned().collect()
    }

    /// Get access statistics
    pub async fn get_stats(&self) -> BaitStats {
        let wallets = self.wallets.read().await;
        let access_log = self.access_log.read().await;

        let total_accesses = access_log.len();
        let unique_attackers: std::collections::HashSet<_> =
            access_log.iter().map(|e| &e.attacker_ip).collect();

        let transaction_attempts = access_log
            .iter()
            .filter(|e| e.transaction_attempted)
            .count();

        BaitStats {
            active_wallets: wallets.len(),
            total_accesses,
            unique_attackers: unique_attackers.len(),
            transaction_attempts,
            last_access: access_log.last().map(|e| e.timestamp),
        }
    }

    /// Export access log for forensics
    pub async fn export_access_log(&self) -> Result<String> {
        let log = self.access_log.read().await;
        Ok(serde_json::to_string_pretty(&*log)?)
    }

    /// Deactivate a bait wallet
    pub async fn deactivate(&self, wallet_id: &str) {
        if let Some(wallet) = self.wallets.write().await.get_mut(wallet_id) {
            wallet.active = false;
            tracing::info!("🎣 Bait wallet {} deactivated", wallet_id);
        }
    }
}

/// Bait statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaitStats {
    pub active_wallets: usize,
    pub total_accesses: usize,
    pub unique_attackers: usize,
    pub transaction_attempts: usize,
    pub last_access: Option<DateTime<Utc>>,
}

/// Create canary tokens (files that call home when opened)
pub struct CanaryToken {
    pub id: String,
    pub token_type: CanaryType,
    pub callback_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CanaryType {
    /// PDF document
    PdfDocument,
    /// Word document
    WordDocument,
    /// Excel spreadsheet
    ExcelSpreadsheet,
    /// URL/Link
    WebLink,
    /// DNS token
    DnsToken,
    /// AWS credentials
    AwsCredentials,
    /// Crypto wallet seed
    WalletSeed,
}

impl CanaryToken {
    /// Create new canary token
    pub fn new(token_type: CanaryType, callback_url: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            token_type,
            callback_url: callback_url.to_string(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bait_wallet_deployment() {
        let manager = BaitWalletManager::new("https://example.com/callback");

        let wallet = manager.deploy_bait(WalletType::Bitcoin, "1.5 BTC").await.unwrap();

        assert!(!wallet.address.is_empty());
        assert!(wallet.address.starts_with("bc1q"));
        assert!(wallet.active);
        println!("✅ Bait wallet deployment test PASSED!");
        println!("   Address: {}", wallet.address);
    }

    #[tokio::test]
    async fn test_honeypot_cluster() {
        let manager = BaitWalletManager::new("https://example.com/callback");

        let wallets = manager.deploy_honeypot_cluster().await.unwrap();

        assert_eq!(wallets.len(), 4);
        println!("✅ Honeypot cluster deployment test PASSED!");
        println!("   Deployed {} bait wallets", wallets.len());
    }

    #[tokio::test]
    async fn test_access_tracking() {
        let manager = BaitWalletManager::new("https://example.com/callback");

        let wallet = manager.deploy_bait(WalletType::Ethereum, "10 ETH").await.unwrap();

        // Simulate access
        manager.handle_access(
            &wallet.id,
            "192.168.1.100",
            AccessType::BalanceCheck,
            Some("Mozilla/5.0"),
        ).await.unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_accesses, 1);
        println!("✅ Access tracking test PASSED!");
    }
}
