use crate::models::Config;
use anyhow::Result;
use std::path::PathBuf;

pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    
    let vibe_dir = config_dir.join(".vibe");
    std::fs::create_dir_all(&vibe_dir)?;
    
    Ok(vibe_dir)
}

pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    } else {
        let default_config = Config::default();
        save_config(&default_config)?;
        Ok(default_config)
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    config.validate()?;
    
    let config_path = get_config_path()?;
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, contents)?;
    
    Ok(())
}