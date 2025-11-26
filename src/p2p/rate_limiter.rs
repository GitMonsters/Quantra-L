use governor::{Quota, RateLimiter as GovernorRateLimiter, clock::DefaultClock, state::{InMemoryState, NotKeyed}};
use libp2p::{PeerId, Multiaddr, multiaddr::Protocol};
use nonzero_ext::*;
use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU32;

/// Rate limiter for P2P connections and messages
pub struct RateLimiter {
    // Global connection rate limit (per IP)
    connection_limiter: HashMap<IpAddr, GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,

    // Per-peer message rate limit
    message_limiter: HashMap<PeerId, GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,

    // Configuration
    connections_per_minute: u32,
    messages_per_second: u32,
}

impl RateLimiter {
    pub fn new(connections_per_minute: u32, messages_per_second: u32) -> Self {
        Self {
            connection_limiter: HashMap::new(),
            message_limiter: HashMap::new(),
            connections_per_minute,
            messages_per_second,
        }
    }

    /// Check if a new connection from this IP is allowed
    pub fn check_connection(&mut self, remote_addr: &Multiaddr) -> bool {
        if let Some(ip) = extract_ip(remote_addr) {
            let limiter = self.connection_limiter.entry(ip).or_insert_with(|| {
                GovernorRateLimiter::direct(
                    Quota::per_minute(
                        NonZeroU32::new(self.connections_per_minute)
                            .unwrap_or(nonzero!(100u32))
                    )
                )
            });

            match limiter.check() {
                Ok(_) => {
                    tracing::debug!("✅ Connection rate limit OK for IP: {}", ip);
                    true
                }
                Err(_) => {
                    tracing::warn!("🚫 Connection rate limit exceeded for IP: {}", ip);
                    false
                }
            }
        } else {
            // If we can't extract IP, be conservative and allow
            true
        }
    }

    /// Check if a message from this peer is allowed
    pub fn check_message(&mut self, peer_id: &PeerId) -> bool {
        let limiter = self.message_limiter.entry(*peer_id).or_insert_with(|| {
            GovernorRateLimiter::direct(
                Quota::per_second(
                    NonZeroU32::new(self.messages_per_second)
                        .unwrap_or(nonzero!(10u32))
                )
            )
        });

        match limiter.check() {
            Ok(_) => {
                tracing::debug!("✅ Message rate limit OK for peer: {}", peer_id);
                true
            }
            Err(_) => {
                tracing::warn!("🚫 Message rate limit exceeded for peer: {}", peer_id);
                false
            }
        }
    }

    /// Register a new peer for message rate limiting
    pub fn register_peer(&mut self, peer_id: PeerId) {
        self.message_limiter.entry(peer_id).or_insert_with(|| {
            GovernorRateLimiter::direct(
                Quota::per_second(
                    NonZeroU32::new(self.messages_per_second)
                        .unwrap_or(nonzero!(10u32))
                )
            )
        });
        tracing::debug!("📝 Registered peer for rate limiting: {}", peer_id);
    }

    /// Unregister a peer (cleanup)
    pub fn unregister_peer(&mut self, peer_id: &PeerId) {
        self.message_limiter.remove(peer_id);
        tracing::debug!("🗑️  Unregistered peer from rate limiting: {}", peer_id);
    }

    /// Clean up old limiters (for IPs that haven't been seen in a while)
    pub fn cleanup(&mut self) {
        // Remove limiters with no recent activity
        // This prevents memory growth from never-seen-again IPs
        // For now, keep all limiters (they're cheap)
        // In production, implement LRU cache or time-based cleanup
    }
}

/// Extract IP address from multiaddress
fn extract_ip(addr: &Multiaddr) -> Option<IpAddr> {
    for component in addr.iter() {
        match component {
            Protocol::Ip4(ip) => return Some(IpAddr::V4(ip)),
            Protocol::Ip6(ip) => return Some(IpAddr::V6(ip)),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_rate_limiter_connection() {
        let mut limiter = RateLimiter::new(5, 10);  // 5 conn/min
        let addr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/9000").unwrap();

        // First 5 connections should succeed
        for i in 0..5 {
            assert!(limiter.check_connection(&addr), "Connection {} should be allowed", i);
        }

        // 6th connection should be rate limited
        assert!(!limiter.check_connection(&addr), "Connection should be rate limited");
    }

    #[test]
    fn test_rate_limiter_message() {
        let mut limiter = RateLimiter::new(100, 10);  // 10 msg/sec
        let peer_id = PeerId::random();

        limiter.register_peer(peer_id);

        // First 10 messages should succeed
        for i in 0..10 {
            assert!(limiter.check_message(&peer_id), "Message {} should be allowed", i);
        }

        // 11th message should be rate limited
        assert!(!limiter.check_message(&peer_id), "Message should be rate limited");
    }
}
