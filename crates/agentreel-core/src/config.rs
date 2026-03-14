use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// AgentReel configuration, loaded from ~/.agentreel/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory for storing trajectories
    pub trajectory_dir: PathBuf,
    /// Redact secrets before saving (default: true)
    pub redact_by_default: bool,
    /// Default tags added to every trajectory
    pub default_tags: Vec<String>,
    /// Registry URL for push/pull
    pub registry_url: Option<String>,
    /// Proxy settings
    pub proxy: ProxyConfig,
    /// Model cost overrides (model_id -> {input, output} per 1M tokens)
    pub model_costs: HashMap<String, ModelCostOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    /// Fixed port for the proxy (default: random)
    pub port: Option<u16>,
    /// Auto-detect provider from request headers
    pub auto_detect_provider: bool,
    /// Override OpenAI upstream URL
    pub openai_upstream: Option<String>,
    /// Override Anthropic upstream URL
    pub anthropic_upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostOverride {
    pub input: f64,
    pub output: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trajectory_dir: default_trajectory_dir(),
            redact_by_default: true,
            default_tags: Vec::new(),
            registry_url: None,
            proxy: ProxyConfig::default(),
            model_costs: HashMap::new(),
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: None,
            auto_detect_provider: true,
            openai_upstream: None,
            anthropic_upstream: None,
        }
    }
}

impl Config {
    /// Load config from default locations. Merges in order:
    /// 1. Built-in defaults
    /// 2. ~/.agentreel/config.toml (if exists)
    /// 3. ./agentreel.toml (if exists)
    /// 4. Environment variables
    pub fn load() -> Result<Self> {
        let mut config = Config::default();

        // Load global config
        let global_path = config_dir().join("config.toml");
        if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)?;
            config = toml::from_str(&content)?;
        }

        // Load local config (overrides global)
        let local_path = PathBuf::from("agentreel.toml");
        if local_path.exists() {
            let content = std::fs::read_to_string(&local_path)?;
            let local: Config = toml::from_str(&content)?;
            config.merge(local);
        }

        // Environment variable overrides
        if let Ok(dir) = std::env::var("AGENTREEL_TRAJECTORY_DIR") {
            config.trajectory_dir = PathBuf::from(dir);
        }
        if let Ok(url) = std::env::var("AGENTREEL_REGISTRY_URL") {
            config.registry_url = Some(url);
        }
        if let Ok(url) = std::env::var("AGENTREEL_OPENAI_UPSTREAM") {
            config.proxy.openai_upstream = Some(url);
        }
        if let Ok(url) = std::env::var("AGENTREEL_ANTHROPIC_UPSTREAM") {
            config.proxy.anthropic_upstream = Some(url);
        }

        Ok(config)
    }

    /// Load from a specific path
    pub fn load_from(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Save config to the global config file
    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Generate a default config file with comments
    pub fn generate_default() -> String {
        r#"# AgentReel Configuration
# Place this file at ~/.agentreel/config.toml

# Directory for storing trajectories
# trajectory_dir = "~/.agentreel/trajectories"

# Redact secrets before saving (default: true)
# redact_by_default = true

# Default tags added to every trajectory
# default_tags = ["my-project"]

# Registry URL for push/pull (future)
# registry_url = "https://registry.agentreel.dev"

[proxy]
# Fixed port for the proxy (default: random)
# port = 8080

# Auto-detect provider from request headers (default: true)
# auto_detect_provider = true

# Override upstream URLs
# openai_upstream = "https://api.openai.com"
# anthropic_upstream = "https://api.anthropic.com"

# [model_costs.my-custom-model]
# input = 5.0   # USD per 1M tokens
# output = 15.0
"#
        .to_string()
    }

    fn merge(&mut self, other: Config) {
        if other.trajectory_dir != default_trajectory_dir() {
            self.trajectory_dir = other.trajectory_dir;
        }
        if !other.redact_by_default {
            self.redact_by_default = false;
        }
        if !other.default_tags.is_empty() {
            self.default_tags.extend(other.default_tags);
        }
        if other.registry_url.is_some() {
            self.registry_url = other.registry_url;
        }
        if other.proxy.port.is_some() {
            self.proxy.port = other.proxy.port;
        }
        if other.proxy.openai_upstream.is_some() {
            self.proxy.openai_upstream = other.proxy.openai_upstream;
        }
        if other.proxy.anthropic_upstream.is_some() {
            self.proxy.anthropic_upstream = other.proxy.anthropic_upstream;
        }
        self.model_costs.extend(other.model_costs);
    }
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agentreel")
}

fn default_trajectory_dir() -> PathBuf {
    config_dir().join("trajectories")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.redact_by_default);
        assert!(config.default_tags.is_empty());
        assert!(config.proxy.auto_detect_provider);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
trajectory_dir = "/tmp/trajectories"
redact_by_default = false
default_tags = ["test", "ci"]

[proxy]
port = 9090
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.trajectory_dir, PathBuf::from("/tmp/trajectories"));
        assert!(!config.redact_by_default);
        assert_eq!(config.default_tags, vec!["test", "ci"]);
        assert_eq!(config.proxy.port, Some(9090));
    }

    #[test]
    fn test_generate_default() {
        let content = Config::generate_default();
        assert!(content.contains("trajectory_dir"));
        assert!(content.contains("[proxy]"));
    }
}
