//! Configuration loader
//!
//! Loads Dispatcher configuration from YAML files.

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::error::{BuildbotError, Result};

/// Loads configuration from YAML files
pub struct ConfigLoader {
    /// Base directory for configuration
    base_dir: Option<PathBuf>,
}

impl ConfigLoader {
    /// Create a new config loader
    pub fn new() -> Self {
        Self { base_dir: None }
    }

    /// Set the base directory
    pub fn set_base_dir(&mut self, dir: PathBuf) {
        self.base_dir = Some(dir);
    }

    /// Load configuration from a YAML file
    pub async fn load_from_file(&self, path: &Path) -> Result<YamlConfig> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            BuildbotError::Config(format!(
                "Failed to read config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        let config: YamlConfig = serde_yaml::from_str(&content).map_err(|e| {
            BuildbotError::Config(format!(
                "Failed to parse YAML config '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    /// Resolve a relative path against the base directory
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        if let Some(ref base) = self.base_dir {
            base.join(path)
        } else {
            PathBuf::from(path)
        }
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────
// YAML config structures
// ─────────────────────────────────────────────────────────────

/// Top-level configuration (YAML format)
#[derive(Debug, Clone, Deserialize)]
pub struct YamlConfig {
    /// Master section
    #[serde(default)]
    pub master: Option<YamlMasterSection>,

    /// Database section
    #[serde(default)]
    pub database: Option<YamlDatabaseSection>,

    /// Web interface
    #[serde(default)]
    pub www: Option<YamlWwwSection>,
}

/// Master section
#[derive(Debug, Clone, Deserialize)]
pub struct YamlMasterSection {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default = "default_web_url")]
    pub web_url: String,
    /// Strict Python dependency mode for dispatcher
    #[serde(default)]
    pub strict_python_deps: Option<bool>,
    /// Base directory for dispatcher repository storage
    #[serde(default)]
    pub dispatcher_workdir: Option<String>,
    /// Runner heartbeat timeout in seconds
    #[serde(default)]
    pub runner_timeout_secs: Option<i64>,
}

/// Database section
#[derive(Debug, Clone, Deserialize)]
pub struct YamlDatabaseSection {
    #[serde(default)]
    pub url: Option<String>,
}

/// Web interface section
#[derive(Debug, Clone, Deserialize)]
pub struct YamlWwwSection {
    #[serde(default = "default_api_port")]
    pub port: u16,
    #[serde(default)]
    pub web_port: Option<u16>,
}

// ─────────────────────────────────────────────────────────────
// Conversion to MasterConfig
// ─────────────────────────────────────────────────────────────

use crate::master::config::MasterConfig;

impl YamlConfig {
    /// Convert YAML config to MasterConfig
    pub fn into_master_config(self, basedir: PathBuf) -> MasterConfig {
        let master_name = self
            .master
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(default_name);

        let web_url = self
            .master
            .as_ref()
            .and_then(|m| {
                if m.web_url.is_empty() {
                    None
                } else {
                    Some(m.web_url.clone())
                }
            })
            .unwrap_or_else(default_web_url);

        // Determine ports
        let api_port = self
            .www
            .as_ref()
            .map(|w| w.port)
            .unwrap_or_else(default_api_port);
        let web_port = self.www.as_ref().and_then(|w| w.web_port).unwrap_or(8011);

        // Database URL
        let database_url = self
            .database
            .as_ref()
            .and_then(|d| d.url.clone())
            .unwrap_or_else(|| {
                format!("sqlite:{}?mode=rwc", basedir.join("buildbot.db").display())
            });

        // Dispatcher settings
        let dispatcher_workdir = self
            .master
            .as_ref()
            .and_then(|m| m.dispatcher_workdir.clone())
            .map(PathBuf::from)
            .unwrap_or_else(|| basedir.join("dispatcher_repos"));

        let runner_timeout_secs = self
            .master
            .as_ref()
            .and_then(|m| m.runner_timeout_secs)
            .unwrap_or(300);

        MasterConfig {
            name: master_name,
            basedir: basedir.clone(),
            api_port,
            web_port,
            database_url,
            web_url,
            build_complete_callback: None,
            strict_python_deps: self
                .master
                .as_ref()
                .and_then(|m| m.strict_python_deps)
                .unwrap_or(false),
            dispatcher_workdir,
            runner_timeout_secs,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Default values
// ─────────────────────────────────────────────────────────────

fn default_name() -> String {
    "buildbot-dispatcher".to_string()
}

fn default_web_url() -> String {
    "http://localhost:8011".to_string()
}

fn default_api_port() -> u16 {
    8010
}
