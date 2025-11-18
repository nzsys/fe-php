use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WafMode {
    Off,
    Learn,
    Detect,
    Block,
}

impl Default for WafMode {
    fn default() -> Self {
        Self::Off
    }
}

impl fmt::Display for WafMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::Learn => write!(f, "learn"),
            Self::Detect => write!(f, "detect"),
            Self::Block => write!(f, "block"),
        }
    }
}

impl FromStr for WafMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "learn" => Ok(Self::Learn),
            "detect" => Ok(Self::Detect),
            "block" => Ok(Self::Block),
            _ => Err(anyhow::anyhow!("Invalid WAF mode: '{}'. Valid values: off, learn, detect, block", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStrategy {
    AbTest,
    Canary,
}

impl Default for DeploymentStrategy {
    fn default() -> Self {
        Self::AbTest
    }
}

impl fmt::Display for DeploymentStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbTest => write!(f, "ab_test"),
            Self::Canary => write!(f, "canary"),
        }
    }
}

impl FromStr for DeploymentStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ab_test" => Ok(Self::AbTest),
            "canary" => Ok(Self::Canary),
            _ => Err(anyhow::anyhow!("Invalid deployment strategy: '{}'. Valid values: ab_test, canary", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingAlgorithm {
    /// Round-robin distribution
    RoundRobin,
    /// Least connections
    LeastConn,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// IP hash
    IpHash,
}

impl Default for LoadBalancingAlgorithm {
    fn default() -> Self {
        Self::RoundRobin
    }
}

impl fmt::Display for LoadBalancingAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoundRobin => write!(f, "round_robin"),
            Self::LeastConn => write!(f, "least_conn"),
            Self::WeightedRoundRobin => write!(f, "weighted_round_robin"),
            Self::IpHash => write!(f, "ip_hash"),
        }
    }
}

impl FromStr for LoadBalancingAlgorithm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "round_robin" => Ok(Self::RoundRobin),
            "least_conn" => Ok(Self::LeastConn),
            "weighted_round_robin" => Ok(Self::WeightedRoundRobin),
            "ip_hash" => Ok(Self::IpHash),
            _ => Err(anyhow::anyhow!(
                "Invalid load balancing algorithm: '{}'. Valid values: round_robin, least_conn, weighted_round_robin, ip_hash",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListenType {
    Tcp,
    Unix,
}

impl Default for ListenType {
    fn default() -> Self {
        ListenType::Tcp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "lowercase")]
pub enum PathPatternConfig {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Regex(String),
}
