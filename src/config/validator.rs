use super::Config;
use anyhow::Result;

pub fn validate_config(config: &Config) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    // Validate server config
    if config.server.port < 1024 {
        warnings.push(format!(
            "   Port {} requires root privileges. Consider using a port >= 1024",
            config.server.port
        ));
    }

    if config.server.workers == 0 {
        warnings.push("   Worker count is 0. It will default to CPU count.".to_string());
    }

    if config.server.workers > num_cpus::get() * 2 {
        warnings.push(format!(
            "   Worker count ({}) is more than 2x CPU cores ({}). This may cause performance degradation.",
            config.server.workers,
            num_cpus::get()
        ));
    }

    // Validate PHP config
    if !config.php.libphp_path.exists() {
        warnings.push(format!(
            "L libphp.so not found at: {}",
            config.php.libphp_path.display()
        ));
    }

    if !config.php.document_root.exists() {
        warnings.push(format!(
            "L Document root not found: {}",
            config.php.document_root.display()
        ));
    }

    if config.php.worker_pool_size == 0 {
        warnings.push("L PHP worker pool size cannot be 0".to_string());
    }

    if config.php.worker_max_requests == 0 {
        warnings.push("   Worker max requests is 0. Workers will never restart.".to_string());
    }

    // Validate logging
    if !["trace", "debug", "info", "warn", "error"].contains(&config.logging.level.as_str()) {
        warnings.push(format!(
            "L Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
            config.logging.level
        ));
    }

    if !["json", "pretty"].contains(&config.logging.format.as_str()) {
        warnings.push(format!(
            "L Invalid log format: {}. Must be 'json' or 'pretty'",
            config.logging.format
        ));
    }

    // Validate WAF
    if config.waf.enable {
        if !["off", "learn", "detect", "block"].contains(&config.waf.mode.as_str()) {
            warnings.push(format!(
                "L Invalid WAF mode: {}. Must be one of: off, learn, detect, block",
                config.waf.mode
            ));
        }

        if let Some(ref rules_path) = config.waf.rules_path {
            if !rules_path.exists() {
                warnings.push(format!(
                    "L WAF rules file not found: {}",
                    rules_path.display()
                ));
            }
        }

        if config.waf.rate_limit.requests_per_ip == 0 {
            warnings.push("   Rate limit is 0. Rate limiting will be disabled.".to_string());
        }
    }

    // Validate admin
    if config.admin.enable {
        if let Some(parent) = config.admin.unix_socket.parent() {
            if !parent.exists() {
                warnings.push(format!(
                    "   Unix socket directory does not exist: {}",
                    parent.display()
                ));
            }
        }

        if config.admin.http_port == config.server.port {
            warnings.push("L Admin port conflicts with server port".to_string());
        }

        if config.admin.http_port == config.metrics.port {
            warnings.push("L Admin port conflicts with metrics port".to_string());
        }
    }

    // Port conflicts
    if config.metrics.port == config.server.port {
        warnings.push("L Metrics port conflicts with server port".to_string());
    }

    // Generate recommendations
    if config.php.opcache.enable && config.php.opcache.validate_timestamps {
        warnings.push(
            "=¡ Recommendation: Disable opcache.validate_timestamps in production for better performance".to_string()
        );
    }

    if config.logging.level == "debug" || config.logging.level == "trace" {
        warnings.push(
            "=¡ Recommendation: Use 'info' or 'warn' log level in production".to_string()
        );
    }

    if config.waf.enable && config.waf.mode == "off" {
        warnings.push(
            "=¡ WAF is enabled but mode is 'off'. Consider using 'learn', 'detect', or 'block'".to_string()
        );
    }

    Ok(warnings)
}
