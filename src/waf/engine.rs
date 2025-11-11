use super::rules::{WafRule, WafAction, WafField};
use crate::metrics::MetricsCollector;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{warn, info};

pub struct WafEngine {
    rules: Vec<WafRule>,
    mode: String,
    metrics: Arc<MetricsCollector>,
}

impl WafEngine {
    pub fn new(rules: Vec<WafRule>, mode: String, metrics: Arc<MetricsCollector>) -> Self {
        info!("WAF Engine initialized with {} rules in {} mode", rules.len(), mode);
        Self {
            rules,
            mode,
            metrics,
        }
    }

    pub fn check_request(
        &self,
        method: &str,
        uri: &str,
        query_string: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> WafResult {
        if self.mode == "off" {
            return WafResult::Allow;
        }

        let user_agent = headers
            .get("user-agent")
            .or_else(|| headers.get("User-Agent"))
            .map(|s| s.as_str())
            .unwrap_or("");

        for rule in &self.rules {
            let value = match rule.field {
                WafField::Uri => uri,
                WafField::QueryString => query_string,
                WafField::UserAgent => user_agent,
                WafField::Method => method,
                WafField::Headers => {
                    // Check all header values
                    let headers_str = headers.values()
                        .map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if rule.matches(&headers_str) {
                        return self.handle_match(rule);
                    }
                    continue;
                }
                WafField::Body => {
                    let body_str = String::from_utf8_lossy(body);
                    if rule.matches(&body_str) {
                        return self.handle_match(rule);
                    }
                    continue;
                }
            };

            if rule.matches(value) {
                return self.handle_match(rule);
            }
        }

        WafResult::Allow
    }

    fn handle_match(&self, rule: &WafRule) -> WafResult {
        self.metrics.inc_waf_blocked(&rule.id);

        warn!(
            "WAF rule triggered: {} - {}",
            rule.id, rule.description
        );

        match self.mode.as_str() {
            "learn" => {
                info!("WAF Learn mode: Would block rule {}", rule.id);
                WafResult::Allow
            }
            "detect" => {
                info!("WAF Detect mode: Detected rule {}", rule.id);
                WafResult::Allow
            }
            "block" => {
                WafResult::Block(rule.clone())
            }
            _ => WafResult::Allow,
        }
    }
}

pub enum WafResult {
    Allow,
    Block(WafRule),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waf::rules::default_rules;

    #[test]
    fn test_sql_injection_detection() {
        let metrics = Arc::new(MetricsCollector::new());
        let engine = WafEngine::new(default_rules(), "detect".to_string(), metrics);

        let headers = HashMap::new();
        let body = vec![];

        let result = engine.check_request(
            "GET",
            "/test",
            "id=1 UNION SELECT * FROM users",
            &headers,
            &body,
        );

        match result {
            WafResult::Allow => {}  // In detect mode, it logs but allows
            WafResult::Block(_) => panic!("Should not block in detect mode"),
        }
    }

    #[test]
    fn test_xss_detection() {
        let metrics = Arc::new(MetricsCollector::new());
        let engine = WafEngine::new(default_rules(), "block".to_string(), metrics);

        let headers = HashMap::new();
        let body = vec![];

        let result = engine.check_request(
            "GET",
            "/test",
            "comment=<script>alert('xss')</script>",
            &headers,
            &body,
        );

        match result {
            WafResult::Allow => panic!("Should block XSS"),
            WafResult::Block(rule) => {
                assert!(rule.id.starts_with("XSS"));
            }
        }
    }
}
