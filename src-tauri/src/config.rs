use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_scan_concurrency: Option<usize>,
}

const DEFAULT_MAX_SCAN_CONCURRENCY: usize = 4;
const MAX_SCAN_CONCURRENCY: usize = 4;

pub fn scan_concurrency() -> usize {
    let cpu_count = std::thread::available_parallelism().map_or(1, usize::from);
    let memory_bytes = total_memory_bytes();
    let automatic = concurrency_for(cpu_count, memory_bytes);
    let cap = load_config()
        .ok()
        .and_then(|config| config.max_scan_concurrency);
    concurrency_with_cap(automatic, cap)
}

fn concurrency_with_cap(automatic: usize, cap: Option<usize>) -> usize {
    cap.map_or(automatic, |cap| {
        automatic.min(cap.clamp(1, MAX_SCAN_CONCURRENCY))
    })
}

fn concurrency_for(cpu_count: usize, memory_bytes: u64) -> usize {
    let memory_limit = if memory_bytes <= 8 * 1024 * 1024 * 1024 {
        2
    } else if memory_bytes <= 16 * 1024 * 1024 * 1024 {
        3
    } else {
        DEFAULT_MAX_SCAN_CONCURRENCY
    };
    cpu_count.min(memory_limit).clamp(1, MAX_SCAN_CONCURRENCY)
}

#[cfg(windows)]
fn total_memory_bytes() -> u64 {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    let mut status = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    if unsafe { GlobalMemoryStatusEx(&mut status) } != 0 {
        status.total_phys
    } else {
        8 * 1024 * 1024 * 1024
    }
}

#[cfg(not(windows))]
fn total_memory_bytes() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|info| {
            info.lines().find_map(|line| {
                line.strip_prefix("MemTotal:")?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
        })
        .map_or(8 * 1024 * 1024 * 1024, |kib| kib.saturating_mul(1024))
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

    #[test]
    fn adaptive_concurrency_respects_ram_cpu_and_server_cap() {
        const GIB: u64 = 1024 * 1024 * 1024;
        for (ram, expected) in [(8, 2), (16, 3), (32, 4), (64, 4)] {
            for cpu in [4, 8, 16] {
                let expected = expected.min(cpu);
                assert_eq!(
                    concurrency_for(cpu, ram * GIB),
                    expected,
                    "{ram} GiB / {cpu} CPUs"
                );
            }
        }
        assert_eq!(concurrency_for(1, 64 * GIB), 1);
        assert_eq!(concurrency_for(4, 4 * GIB), 2);
    }

    #[test]
    fn config_concurrency_only_lowers_automatic_and_legacy_json_loads() {
        for (value, expected) in [(0, 1), (1, 1), (4, 4), (5, 4), (6, 4), (usize::MAX, 4)] {
            assert_eq!(value.clamp(1, MAX_SCAN_CONCURRENCY), expected);
            let json = format!(r#"{{"max_scan_concurrency":{value}}}"#);
            let config: AppConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(
                concurrency_with_cap(4, config.max_scan_concurrency),
                expected
            );
        }
        assert_eq!(concurrency_with_cap(3, Some(6)), 3);
        assert_eq!(concurrency_with_cap(4, Some(2)), 2);
        assert_eq!(concurrency_with_cap(4, None), 4);
        assert_eq!(
            serde_json::from_str::<AppConfig>(r#"{"server_address":"127.0.0.1:13310"}"#).unwrap(),
            AppConfig {
                server_address: Some("127.0.0.1:13310".into()),
                max_scan_concurrency: None
            }
        );
    }
}
