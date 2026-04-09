//! Master configuration for Dispatcher-based CI system

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Master configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfig {
    /// Master name
    pub name: String,
    /// Base directory
    pub basedir: PathBuf,
    /// API port
    pub api_port: u16,
    /// Web port
    pub web_port: u16,
    /// Database URL
    pub database_url: String,
    /// Bind address for web interface
    pub web_url: String,
    /// URL to POST build completion notifications to
    #[serde(default)]
    pub build_complete_callback: Option<String>,
    /// Strict Python dependency mode for dispatcher (only allow imports in requirements.txt)
    #[serde(default)]
    pub strict_python_deps: bool,
    /// Base directory for dispatcher repository storage
    #[serde(default = "default_dispatcher_workdir")]
    pub dispatcher_workdir: PathBuf,
    /// Runner heartbeat timeout in seconds (default 300 = 5 minutes)
    #[serde(default = "default_runner_timeout")]
    pub runner_timeout_secs: i64,
}

fn default_dispatcher_workdir() -> PathBuf {
    PathBuf::from("dispatcher_repos")
}

fn default_runner_timeout() -> i64 {
    300 // 5 minutes
}
