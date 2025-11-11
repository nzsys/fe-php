use super::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub fn parse_config(path: &PathBuf) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

pub fn save_config(config: &Config, path: &PathBuf) -> Result<()> {
    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;

    fs::write(path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_minimal_config() {
        let config_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[php]
libphp_path = "/usr/local/lib/libphp.so"
document_root = "/var/www/html"

[logging]
level = "info"

[metrics]
enable = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        let path = PathBuf::from(temp_file.path());

        let config = parse_config(&path).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
    }
}
