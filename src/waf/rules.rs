use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafRule {
    pub id: String,
    pub description: String,
    pub pattern: String,
    #[serde(skip)]
    pub regex: Option<Regex>,
    pub field: WafField,
    pub action: WafAction,
    pub severity: WafSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WafField {
    Uri,
    QueryString,
    Headers,
    Body,
    UserAgent,
    Method,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WafAction {
    Block,
    Log,
    Challenge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WafSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl WafRule {
    pub fn new(
        id: String,
        description: String,
        pattern: String,
        field: WafField,
        action: WafAction,
        severity: WafSeverity,
    ) -> Self {
        let regex = Regex::new(&pattern).ok();

        Self {
            id,
            description,
            pattern,
            regex,
            field,
            action,
            severity,
        }
    }

    pub fn matches(&self, value: &str) -> bool {
        if let Some(ref regex) = self.regex {
            regex.is_match(value)
        } else {
            false
        }
    }
}

// OWASP Core Rule Set examples
pub fn default_rules() -> Vec<WafRule> {
    vec![
        // SQL Injection
        WafRule::new(
            "SQL-001".to_string(),
            "SQL Injection - UNION attack".to_string(),
            r"(?i)union.+select".to_string(),
            WafField::QueryString,
            WafAction::Block,
            WafSeverity::Critical,
        ),
        WafRule::new(
            "SQL-002".to_string(),
            "SQL Injection - Comment".to_string(),
            r"(?i)(--|#|/\*|\*/|;)".to_string(),
            WafField::QueryString,
            WafAction::Block,
            WafSeverity::High,
        ),
        // XSS
        WafRule::new(
            "XSS-001".to_string(),
            "XSS - Script tag".to_string(),
            r"(?i)<script[^>]*>.*?</script>".to_string(),
            WafField::QueryString,
            WafAction::Block,
            WafSeverity::High,
        ),
        WafRule::new(
            "XSS-002".to_string(),
            "XSS - Event handler".to_string(),
            r"(?i)on(load|error|click|mouse)".to_string(),
            WafField::QueryString,
            WafAction::Block,
            WafSeverity::High,
        ),
        // Path Traversal
        WafRule::new(
            "PATH-001".to_string(),
            "Path Traversal".to_string(),
            r"\.\.[\\/]".to_string(),
            WafField::Uri,
            WafAction::Block,
            WafSeverity::High,
        ),
        // Command Injection
        WafRule::new(
            "CMD-001".to_string(),
            "Command Injection".to_string(),
            r"(?i)(;|\||&|`|\$\(|\$\{)".to_string(),
            WafField::QueryString,
            WafAction::Block,
            WafSeverity::Critical,
        ),
    ]
}
