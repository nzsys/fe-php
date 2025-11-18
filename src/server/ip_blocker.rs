use parking_lot::RwLock;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::str::FromStr;

/// Dynamic IP blocker for runtime IP blocking/unblocking
#[derive(Clone)]
pub struct IpBlocker {
    blocked_ips: Arc<RwLock<HashSet<IpAddr>>>,
}

impl IpBlocker {
    pub fn new() -> Self {
        Self {
            blocked_ips: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Block an IP address
    pub fn block(&self, ip: &str) -> Result<(), String> {
        match IpAddr::from_str(ip) {
            Ok(addr) => {
                let mut blocked = self.blocked_ips.write();
                blocked.insert(addr);
                Ok(())
            }
            Err(e) => Err(format!("Invalid IP address '{}': {}", ip, e)),
        }
    }

    /// Unblock an IP address
    pub fn unblock(&self, ip: &str) -> Result<(), String> {
        match IpAddr::from_str(ip) {
            Ok(addr) => {
                let mut blocked = self.blocked_ips.write();
                blocked.remove(&addr);
                Ok(())
            }
            Err(e) => Err(format!("Invalid IP address '{}': {}", ip, e)),
        }
    }

    /// Check if an IP is blocked
    pub fn is_blocked(&self, ip: &IpAddr) -> bool {
        let blocked = self.blocked_ips.read();
        blocked.contains(ip)
    }

    /// Get all blocked IPs
    pub fn get_blocked_ips(&self) -> Vec<String> {
        let blocked = self.blocked_ips.read();
        blocked.iter().map(|ip| ip.to_string()).collect()
    }

    /// Get count of blocked IPs
    pub fn count(&self) -> usize {
        let blocked = self.blocked_ips.read();
        blocked.len()
    }

    /// Clear all blocked IPs
    pub fn clear(&self) {
        let mut blocked = self.blocked_ips.write();
        blocked.clear();
    }
}

impl Default for IpBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_unblock() {
        let blocker = IpBlocker::new();

        // Block IP
        assert!(blocker.block("192.168.1.100").is_ok());
        let ip = IpAddr::from_str("192.168.1.100").unwrap();
        assert!(blocker.is_blocked(&ip));

        // Unblock IP
        assert!(blocker.unblock("192.168.1.100").is_ok());
        assert!(!blocker.is_blocked(&ip));
    }

    #[test]
    fn test_invalid_ip() {
        let blocker = IpBlocker::new();
        assert!(blocker.block("invalid_ip").is_err());
    }

    #[test]
    fn test_get_blocked_ips() {
        let blocker = IpBlocker::new();
        blocker.block("192.168.1.100").unwrap();
        blocker.block("10.0.0.1").unwrap();

        let blocked = blocker.get_blocked_ips();
        assert_eq!(blocked.len(), 2);
        assert!(blocked.contains(&"192.168.1.100".to_string()) || blocked.contains(&"10.0.0.1".to_string()));
    }

    #[test]
    fn test_clear() {
        let blocker = IpBlocker::new();
        blocker.block("192.168.1.100").unwrap();
        blocker.block("10.0.0.1").unwrap();
        assert_eq!(blocker.count(), 2);

        blocker.clear();
        assert_eq!(blocker.count(), 0);
    }
}
