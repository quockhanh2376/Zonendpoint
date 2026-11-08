use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfigDto {
    pub effective_addr: String,
    pub source: String, // "env" | "saved" | "default"
    pub saved_addr: Option<String>,
    pub env_override: bool,
}

pub fn config_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "LOCALAPPDATA is unavailable".to_string())?;
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or_else(|| "user config directory is unavailable".to_string())?;
    Ok(base.join("Zonendpoint"))
}

pub fn config_path() -> Result<PathBuf, String> {
    if let Some(custom) = std::env::var_os("ZONENDPOINT_CONFIG_FILE") {
        return Ok(PathBuf::from(custom));
    }
    Ok(config_dir()?.join("config.json"))
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read config file {}: {}", path.display(), e))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("failed to parse config file {}: {}", path.display(), e))
}

pub fn load_saved_server_address() -> Result<Option<String>, String> {
    load_config().map(|c| c.server_address)
}

pub fn save_server_address(addr: &str) -> Result<(), String> {
    let trimmed = addr.trim();
    let validated_addr = if trimmed.is_empty() {
        None
    } else {
        crate::scanner::validate_clamd_addr(trimmed)?;
        Some(trimmed.to_string())
    };

    let mut config = load_config().unwrap_or_default();
    config.server_address = validated_addr;

    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config dir {}: {}", parent.display(), e))?;
    }

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("failed to serialize config: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("failed to write config to {}: {}", path.display(), e))?;
    Ok(())
}

pub fn get_server_config() -> Result<ServerConfigDto, String> {
    let saved = load_saved_server_address().ok().flatten();

    // Priority: 1) ZONENDPOINT_CLAMD > 2) saved config > 3) default 127.0.0.1:13310
    if let Ok(env_val) = std::env::var("ZONENDPOINT_CLAMD") {
        let trimmed = env_val.trim();
        if !trimmed.is_empty() {
            return Ok(ServerConfigDto {
                effective_addr: trimmed.to_string(),
                source: "env".to_string(),
                saved_addr: saved,
                env_override: true,
            });
        }
    }

    if let Some(ref saved_addr) = saved {
        let trimmed = saved_addr.trim();
        if !trimmed.is_empty() {
            return Ok(ServerConfigDto {
                effective_addr: trimmed.to_string(),
                source: "saved".to_string(),
                saved_addr: saved,
                env_override: false,
            });
        }
    }

    Ok(ServerConfigDto {
        effective_addr: crate::scanner::DEFAULT_CLAMD_ADDR.to_string(),
        source: "default".to_string(),
        saved_addr: saved,
        env_override: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_to_none_when_file_absent() {
        let custom_file =
            std::env::temp_dir().join(format!("zonendpoint-cfg-test-{}", uuid::Uuid::new_v4()));
        let mut config = AppConfig::default();
        assert_eq!(config.server_address, None);
        config.server_address = Some("127.0.0.1:13310".into());
        let json = serde_json::to_string(&config).unwrap();
        std::fs::write(&custom_file, json).unwrap();
        let loaded: AppConfig =
            serde_json::from_str(&std::fs::read_to_string(&custom_file).unwrap()).unwrap();
        assert_eq!(loaded.server_address.as_deref(), Some("127.0.0.1:13310"));
        let _ = std::fs::remove_file(custom_file);
    }
}
