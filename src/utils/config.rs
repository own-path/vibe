use crate::models::Config;
use anyhow::Result;
use std::path::PathBuf;

pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    let tempo_dir = config_dir.join(".tempo");
    std::fs::create_dir_all(&tempo_dir)?;

    Ok(tempo_dir)
}

pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse config file: {}. Please check the file format.",
                e
            )
        })?;

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
    let mut contents = toml::to_string_pretty(config)?;

    // Remove empty [custom_settings] section if it was written (handle various formats)
    if config.custom_settings.is_empty() {
        // Remove standalone [custom_settings] line
        contents = contents.replace("[custom_settings]\n", "");
        contents = contents.replace("\n[custom_settings]", "");
        // Remove [custom_settings] at the end of file
        contents = contents.replace("\n[custom_settings]", "");
        // Clean up any double newlines
        contents = contents.replace("\n\n\n", "\n\n");
    }

    std::fs::write(&config_path, contents.trim_end().to_string() + "\n")?;

    Ok(())
}
