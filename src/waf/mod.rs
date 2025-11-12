pub mod engine;
pub mod rules;

pub use engine::WafEngine;
pub use rules::{WafRule, WafAction, WafSeverity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    pub enable: bool,
    pub mode: String,  // off, learn, detect, block
    pub rules: Vec<WafRule>,
}

impl WafConfig {
    pub fn new() -> Self {
        Self {
            enable: false,
            mode: "off".to_string(),
            rules: Vec::new(),
        }
    }
}

impl Default for WafConfig {
    fn default() -> Self {
        Self::new()
    }
}
