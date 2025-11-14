use ipnetwork::IpNetwork;
use std::net::IpAddr;
use std::str::FromStr;

/// IP filtering result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpFilterDecision {
    Allow,
    Deny,
}

/// IP filter with whitelist and blacklist support
pub struct IpFilter {
    allowed_networks: Vec<IpNetwork>,
    denied_networks: Vec<IpNetwork>,
    mode: FilterMode,
}

#[derive(Debug, Clone, Copy)]
enum FilterMode {
    Whitelist, // Only allowed IPs can access
    Blacklist, // All except denied IPs can access
}

impl IpFilter {
    /// Create filter in whitelist mode
    pub fn whitelist(allowed_cidrs: Vec<String>) -> Result<Self, String> {
        let networks = Self::parse_networks(allowed_cidrs)?;
        Ok(Self {
            allowed_networks: networks,
            denied_networks: Vec::new(),
            mode: FilterMode::Whitelist,
        })
    }

    /// Create filter in blacklist mode
    pub fn blacklist(denied_cidrs: Vec<String>) -> Result<Self, String> {
        let networks = Self::parse_networks(denied_cidrs)?;
        Ok(Self {
            allowed_networks: Vec::new(),
            denied_networks: networks,
            mode: FilterMode::Blacklist,
        })
    }

    /// Create filter with both allow and deny lists
    pub fn combined(allowed_cidrs: Vec<String>, denied_cidrs: Vec<String>) -> Result<Self, String> {
        let allowed = Self::parse_networks(allowed_cidrs)?;
        let denied = Self::parse_networks(denied_cidrs)?;

        Ok(Self {
            allowed_networks: allowed,
            denied_networks: denied,
            mode: FilterMode::Blacklist,
        })
    }

    fn parse_networks(cidrs: Vec<String>) -> Result<Vec<IpNetwork>, String> {
        cidrs.into_iter()
            .map(|cidr| IpNetwork::from_str(&cidr).map_err(|e| format!("Invalid CIDR '{}': {}", cidr, e)))
            .collect()
    }

    /// Check if an IP address is allowed
    pub fn check(&self, ip: IpAddr) -> IpFilterDecision {
        // Check deny list first
        for network in &self.denied_networks {
            if network.contains(ip) {
                return IpFilterDecision::Deny;
            }
        }

        match self.mode {
            FilterMode::Whitelist => {
                // In whitelist mode, IP must be in allowed list
                for network in &self.allowed_networks {
                    if network.contains(ip) {
                        return IpFilterDecision::Allow;
                    }
                }
                IpFilterDecision::Deny
            }
            FilterMode::Blacklist => {
                // In blacklist mode, if not denied, allow
                IpFilterDecision::Allow
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitelist_mode() {
        let filter = IpFilter::whitelist(vec![
            "192.168.1.0/24".to_string(),
            "10.0.0.0/8".to_string(),
        ]).unwrap();

        assert_eq!(filter.check("192.168.1.100".parse().unwrap()), IpFilterDecision::Allow);
        assert_eq!(filter.check("10.1.2.3".parse().unwrap()), IpFilterDecision::Allow);
        assert_eq!(filter.check("1.2.3.4".parse().unwrap()), IpFilterDecision::Deny);
    }

    #[test]
    fn test_blacklist_mode() {
        let filter = IpFilter::blacklist(vec![
            "1.2.3.4/32".to_string(),
        ]).unwrap();

        assert_eq!(filter.check("1.2.3.4".parse().unwrap()), IpFilterDecision::Deny);
        assert_eq!(filter.check("192.168.1.1".parse().unwrap()), IpFilterDecision::Allow);
    }
}
