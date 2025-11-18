use serde::{Deserialize, Serialize};
use super::defaults::*;
use super::types::{LoadBalancingAlgorithm, DeploymentStrategy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_redis_url")]
    pub url: String,
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: u32,
    #[serde(default = "default_redis_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_redis_prefix")]
    pub key_prefix: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            enable: false,
            url: default_redis_url(),
            pool_size: default_redis_pool_size(),
            timeout_ms: default_redis_timeout(),
            key_prefix: default_redis_prefix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enable: false,
            otlp_endpoint: default_otlp_endpoint(),
            service_name: default_service_name(),
            sample_rate: default_sample_rate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,
    #[serde(default)]
    pub algorithm: LoadBalancingAlgorithm,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            enable: false,
            upstreams: Vec::new(),
            algorithm: LoadBalancingAlgorithm::default(),
            health_check: HealthCheckConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    pub url: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_health_check_path")]
    pub path: String,
    #[serde(default = "default_health_check_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_health_check_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enable: true,
            path: default_health_check_path(),
            interval_seconds: default_health_check_interval(),
            timeout_seconds: default_health_check_timeout(),
            unhealthy_threshold: default_unhealthy_threshold(),
            healthy_threshold: default_healthy_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: usize,
    #[serde(default = "default_success_threshold")]
    pub success_threshold: usize,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_half_open_max_requests")]
    pub half_open_max_requests: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enable: false,
            failure_threshold: default_failure_threshold(),
            success_threshold: default_success_threshold(),
            timeout_seconds: default_timeout_seconds(),
            half_open_max_requests: default_half_open_max_requests(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub strategy: DeploymentStrategy,
    #[serde(default)]
    pub variants: Vec<VariantConfig>,
    #[serde(default)]
    pub sticky_sessions: bool,
    #[serde(default)]
    pub ab_test: AbTestConfig,
    #[serde(default)]
    pub canary: CanaryConfig,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            enable: false,
            strategy: DeploymentStrategy::default(),
            variants: Vec::new(),
            sticky_sessions: true,
            ab_test: AbTestConfig::default(),
            canary: CanaryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantConfig {
    pub name: String,
    pub weight: u32,
    pub upstream: String,
    #[serde(default = "default_true")]
    pub metrics_tracking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTestConfig {
    #[serde(default = "default_true")]
    pub track_conversion: bool,
    #[serde(default = "default_min_requests")]
    pub min_requests_per_variant: u64,
}

impl Default for AbTestConfig {
    fn default() -> Self {
        Self {
            track_conversion: true,
            min_requests_per_variant: default_min_requests(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    #[serde(default = "default_max_error_rate")]
    pub max_error_rate: f64,
    #[serde(default)]
    pub max_response_time_ms: Option<u64>,
    #[serde(default = "default_min_observation_period")]
    pub min_observation_period_secs: u64,
    #[serde(default = "default_min_requests")]
    pub min_requests_before_decision: u64,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            max_error_rate: default_max_error_rate(),
            max_response_time_ms: None,
            min_observation_period_secs: default_min_observation_period(),
            min_requests_before_decision: default_min_requests(),
        }
    }
}
