use super::{Backend, BackendError, BackendType, PathPattern};
use crate::config::{PathPatternConfig, RoutingRule};
use crate::metrics::MetricsCollector;
use crate::php::{PhpRequest, PhpResponse};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct BackendRouter {
    backends: HashMap<BackendType, Arc<dyn Backend>>,
    rules: Vec<CompiledRoutingRule>,
    default_backend: BackendType,
}

struct CompiledRoutingRule {
    pattern: PathPattern,
    backend_type: BackendType,
    priority: u32,
}

impl BackendRouter {
    pub fn new(
        backends: HashMap<BackendType, Arc<dyn Backend>>,
        routing_rules: Vec<RoutingRule>,
        default_backend: BackendType,
    ) -> Result<Self> {
        let mut rules: Vec<CompiledRoutingRule> = Vec::new();

        for rule in routing_rules {
            let pattern = Self::compile_pattern(&rule.pattern)?;
            let backend_type = rule.backend.parse::<BackendType>()
                .with_context(|| format!("Invalid backend type: {}", rule.backend))?;

            rules.push(CompiledRoutingRule {
                pattern,
                backend_type,
                priority: rule.priority,
            });
        }

        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(Self {
            backends,
            rules,
            default_backend,
        })
    }

    fn compile_pattern(config: &PathPatternConfig) -> Result<PathPattern> {
        match config {
            PathPatternConfig::Exact(s) => Ok(PathPattern::Exact(s.clone())),
            PathPatternConfig::Prefix(s) => Ok(PathPattern::Prefix(s.clone())),
            PathPatternConfig::Suffix(s) => Ok(PathPattern::Suffix(s.clone())),
            PathPatternConfig::Regex(s) => {
                let regex = regex::Regex::new(s)
                    .with_context(|| format!("Invalid regex pattern: {}", s))?;
                Ok(PathPattern::Regex(regex))
            }
        }
    }

    pub fn route(&self, path: &str) -> Arc<dyn Backend> {
        for rule in &self.rules {
            if rule.pattern.matches(path) {
                if let Some(backend) = self.backends.get(&rule.backend_type) {
                    return Arc::clone(backend);
                }
            }
        }

        self.backends
            .get(&self.default_backend)
            .expect("Default backend must exist")
            .clone()
    }

    pub fn backends(&self) -> &HashMap<BackendType, Arc<dyn Backend>> {
        &self.backends
    }

    pub fn rules(&self) -> Vec<(String, BackendType, u32)> {
        self.rules
            .iter()
            .map(|rule| {
                let pattern_desc = match &rule.pattern {
                    PathPattern::Exact(s) => format!("exact:{}", s),
                    PathPattern::Prefix(s) => format!("prefix:{}", s),
                    PathPattern::Suffix(s) => format!("suffix:{}", s),
                    PathPattern::Regex(r) => format!("regex:{}", r.as_str()),
                };
                (pattern_desc, rule.backend_type, rule.priority)
            })
            .collect()
    }

    pub fn execute_with_metrics(
        &self,
        request: PhpRequest,
        metrics: Option<&MetricsCollector>,
    ) -> Result<PhpResponse, BackendError> {
        let path = &request.uri.clone();
        let backend = self.route(path);
        let backend_type = backend.backend_type();
        let backend_name = backend_type.to_string();

        let start = Instant::now();
        let result = backend.execute(request);
        let duration = start.elapsed().as_secs_f64();

        if let Some(metrics) = metrics {
            match &result {
                Ok(_) => {
                    metrics.record_backend_request(&backend_name, "success", duration);
                }
                Err(e) => {
                    let error_type = match e {
                        BackendError::NotFound(_) => "not_found",
                        BackendError::PhpError(_) => "php_error",
                        BackendError::ConnectionFailed(_) => "connection_failed",
                        BackendError::ProtocolError(_) => "protocol_error",
                        BackendError::IoError(_) => "io_error",
                        BackendError::Timeout => "timeout",
                        BackendError::Other(_) => "other",
                    };
                    metrics.record_backend_request(&backend_name, "error", duration);
                    metrics.record_backend_error(&backend_name, error_type);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{BackendError, HealthStatus};
    use crate::php::{PhpRequest, PhpResponse};

    struct MockBackend {
        backend_type: BackendType,
    }

    impl Backend for MockBackend {
        fn execute(&self, _request: PhpRequest) -> Result<PhpResponse, BackendError> {
            Ok(PhpResponse {
                status_code: 200,
                headers: Default::default(),
                body: Vec::new(),
                execution_time_ms: 0,
                memory_peak_mb: 0.0,
            })
        }

        fn health_check(&self) -> Result<HealthStatus> {
            Ok(HealthStatus::healthy("Mock backend"))
        }

        fn backend_type(&self) -> BackendType {
            self.backend_type
        }
    }

    #[test]
    fn test_backend_router_prefix() {
        let mut backends = HashMap::new();
        backends.insert(
            BackendType::Embedded,
            Arc::new(MockBackend {
                backend_type: BackendType::Embedded,
            }) as Arc<dyn Backend>,
        );
        backends.insert(
            BackendType::Static,
            Arc::new(MockBackend {
                backend_type: BackendType::Static,
            }) as Arc<dyn Backend>,
        );

        let rules = vec![RoutingRule {
            pattern: PathPatternConfig::Prefix("/static/*".to_string()),
            backend: "static".to_string(),
            priority: 100,
        }];

        let router =
            BackendRouter::new(backends, rules, BackendType::Embedded).unwrap();

        assert_eq!(
            router.route("/static/image.png").backend_type(),
            BackendType::Static
        );
        assert_eq!(
            router.route("/api/user").backend_type(),
            BackendType::Embedded
        );
    }

    #[test]
    fn test_backend_router_priority() {
        let mut backends = HashMap::new();
        backends.insert(
            BackendType::Embedded,
            Arc::new(MockBackend {
                backend_type: BackendType::Embedded,
            }) as Arc<dyn Backend>,
        );
        backends.insert(
            BackendType::FastCGI,
            Arc::new(MockBackend {
                backend_type: BackendType::FastCGI,
            }) as Arc<dyn Backend>,
        );

        let rules = vec![
            RoutingRule {
                pattern: PathPatternConfig::Prefix("/api/*".to_string()),
                backend: "embedded".to_string(),
                priority: 100,
            },
            RoutingRule {
                pattern: PathPatternConfig::Prefix("/api/*".to_string()),
                backend: "fastcgi".to_string(),
                priority: 50,
            },
        ];

        let router =
            BackendRouter::new(backends, rules, BackendType::Embedded).unwrap();

        assert_eq!(
            router.route("/api/user").backend_type(),
            BackendType::Embedded
        );
    }
}
