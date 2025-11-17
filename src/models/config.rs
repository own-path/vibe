use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub idle_timeout_minutes: u32,
    pub auto_pause_enabled: bool,
    pub default_context: String,
    pub max_session_hours: u32,
    pub backup_enabled: bool,
    pub log_level: String,
    pub custom_settings: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timeout_minutes: 30,
            auto_pause_enabled: true,
            default_context: "terminal".to_string(),
            max_session_hours: 48,
            backup_enabled: true,
            log_level: "info".to_string(),
            custom_settings: HashMap::new(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.idle_timeout_minutes == 0 {
            return Err(anyhow::anyhow!("Idle timeout must be greater than 0"));
        }

        if self.max_session_hours == 0 {
            return Err(anyhow::anyhow!("Max session hours must be greater than 0"));
        }

        let valid_contexts = ["terminal", "ide", "linked", "manual"];
        if !valid_contexts.contains(&self.default_context.as_str()) {
            return Err(anyhow::anyhow!(
                "Default context must be one of: {}",
                valid_contexts.join(", ")
            ));
        }

        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            return Err(anyhow::anyhow!(
                "Log level must be one of: {}",
                valid_levels.join(", ")
            ));
        }

        Ok(())
    }

    pub fn set_custom(&mut self, key: String, value: String) {
        self.custom_settings.insert(key, value);
    }

    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom_settings.get(key)
    }
}