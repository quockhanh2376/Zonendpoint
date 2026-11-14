use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MAX_STATUS_FILE_BYTES: u64 = 64 * 1024; // 64 KB
pub const MAX_SIGNATURES: u64 = 1_000_000_000;
pub const STALE_THRESHOLD: Duration = Duration::from_secs(2 * 3600); // 2 hours

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ServerStatusDto {
    pub signatures: Option<u64>,
    pub loaded_at_utc: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Deserialize)]
struct RawServerStatus {
    signatures: Option<serde_json::Value>,
    loaded_at_utc: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    engine: Option<String>,
}

pub fn parse_iso8601_utc(s: &str) -> Option<u64> {
    let s = s.trim();
    if !s.ends_with('Z') {
        return None;
    }
    let body = &s[..s.len() - 1];
    let parts: Vec<&str> = body.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }
    let year: i64 = date_parts[0].parse().ok()?;
    let month: u32 = date_parts[1].parse().ok()?;
    let day: u32 = date_parts[2].parse().ok()?;
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let sec_str = time_parts[2].split('.').next()?;
    let sec: u32 = sec_str.parse().ok()?;

    if !(1970..=2200).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || sec > 60
    {
        return None;
    }

    fn is_leap(y: i64) -> bool {
        (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
    }

    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => return None,
    };
    if day > days_in_month {
        return None;
    }

    let mut total_days: i64 = 0;
    for y in 1970..year {
        total_days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30];
    for m in 1..month {
        total_days += month_days[m as usize];
        if m == 2 && is_leap(year) {
            total_days += 1;
        }
    }
    total_days += (day - 1) as i64;

    let total_secs =
        total_days * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + (sec as i64);
    if total_secs < 0 {
        None
    } else {
        Some(total_secs as u64)
    }
}

pub fn parse_server_status_content(
    content: &str,
    file_size: u64,
    now: SystemTime,
) -> ServerStatusDto {
    if file_size > MAX_STATUS_FILE_BYTES || content.len() as u64 > MAX_STATUS_FILE_BYTES {
        return ServerStatusDto::default();
    }

    let raw: RawServerStatus = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return ServerStatusDto::default(),
    };

    let sig_val = match raw.signatures {
        Some(serde_json::Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                u
            } else {
                return ServerStatusDto::default();
            }
        }
        _ => return ServerStatusDto::default(),
    };

    if sig_val > MAX_SIGNATURES {
        return ServerStatusDto::default();
    }

    let ts_str = match raw.loaded_at_utc {
        Some(s) if !s.trim().is_empty() => s,
        _ => return ServerStatusDto::default(),
    };

    let loaded_secs = match parse_iso8601_utc(&ts_str) {
        Some(s) => s,
        None => return ServerStatusDto::default(),
    };

    let now_secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let stale = if now_secs >= loaded_secs {
        (now_secs - loaded_secs) >= STALE_THRESHOLD.as_secs()
    } else {
        // Skewed into future by > 300 seconds
        (loaded_secs - now_secs) > 300
    };

    ServerStatusDto {
        signatures: Some(sig_val),
        loaded_at_utc: Some(ts_str),
        stale,
    }
}

pub fn server_status_path() -> Result<PathBuf, String> {
    if let Some(custom) = std::env::var_os("ZONENDPOINT_SERVER_STATUS_FILE") {
        return Ok(PathBuf::from(custom));
    }
    let config_dir = crate::config::config_dir()?;
    let direct = config_dir.join("server-status.json");
    if direct.exists() {
        return Ok(direct);
    }
    // Also check case variation if on case-sensitive filesystem
    if let Some(parent) = config_dir.parent() {
        let alt = parent.join("ZonEndpoint").join("server-status.json");
        if alt.exists() {
            return Ok(alt);
        }
    }
    Ok(direct)
}

pub fn read_server_status_from_path(path: &Path, now: SystemTime) -> ServerStatusDto {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return ServerStatusDto::default(),
    };

    if metadata.len() > MAX_STATUS_FILE_BYTES {
        return ServerStatusDto::default();
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return ServerStatusDto::default(),
    };

    parse_server_status_content(&content, metadata.len(), now)
}

pub fn load_server_status() -> ServerStatusDto {
    match server_status_path() {
        Ok(path) => read_server_status_from_path(&path, SystemTime::now()),
        Err(_) => ServerStatusDto::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time_from_secs(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn test_missing_or_empty_content_returns_empty_dto() {
        let now = SystemTime::now();
        assert_eq!(
            parse_server_status_content("", 0, now),
            ServerStatusDto::default()
        );
        assert_eq!(
            read_server_status_from_path(Path::new("non_existent_file_xyz.json"), now),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_corrupted_json_returns_empty_dto() {
        let now = SystemTime::now();
        assert_eq!(
            parse_server_status_content("{ invalid json", 14, now),
            ServerStatusDto::default()
        );
        assert_eq!(
            parse_server_status_content("null", 4, now),
            ServerStatusDto::default()
        );
        assert_eq!(
            parse_server_status_content("[]", 2, now),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_negative_or_invalid_signatures_number() {
        let now = SystemTime::now();
        // Negative number
        let json_neg = r#"{"signatures": -5, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_neg, json_neg.len() as u64, now),
            ServerStatusDto::default()
        );

        // Float number
        let json_float = r#"{"signatures": 123.45, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_float, json_float.len() as u64, now),
            ServerStatusDto::default()
        );

        // String number
        let json_str = r#"{"signatures": "8616464", "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_str, json_str.len() as u64, now),
            ServerStatusDto::default()
        );

        // Number exceeds 1e9
        let json_overflow =
            r#"{"signatures": 1000000001, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_overflow, json_overflow.len() as u64, now),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_file_too_large_rejected() {
        let now = SystemTime::now();
        let json = r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json, MAX_STATUS_FILE_BYTES + 1, now),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_valid_fresh_signatures() {
        // 2026-11-14T03:39:43Z in unix secs:
        let loaded_secs = parse_iso8601_utc("2026-11-14T03:39:43Z").expect("valid iso");
        // Test time 30 mins later
        let now = time_from_secs(loaded_secs + 1800);

        let json = r#"{
            "signatures": 8616464,
            "loaded_at_utc": "2026-11-14T03:39:43Z",
            "engine": "ClamAV 1.5.4/28182"
        }"#;

        let dto = parse_server_status_content(json, json.len() as u64, now);
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at_utc: Some("2026-11-14T03:39:43Z".to_string()),
                stale: false,
            }
        );
    }

    #[test]
    fn test_valid_stale_signatures_after_two_hours() {
        let loaded_secs = parse_iso8601_utc("2026-11-14T03:39:43Z").expect("valid iso");
        // Exactly 2 hours and 1 second later
        let now = time_from_secs(loaded_secs + 7201);

        let json = r#"{
            "signatures": 8616464,
            "loaded_at_utc": "2026-11-14T03:39:43Z",
            "engine": "ClamAV 1.5.4/28182"
        }"#;

        let dto = parse_server_status_content(json, json.len() as u64, now);
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at_utc: Some("2026-11-14T03:39:43Z".to_string()),
                stale: true,
            }
        );
    }

    #[test]
    fn test_invalid_loaded_at_utc_timestamp() {
        let now = SystemTime::now();
        let json_bad_time = r#"{"signatures": 8616464, "loaded_at_utc": "not-a-timestamp"}"#;
        assert_eq!(
            parse_server_status_content(json_bad_time, json_bad_time.len() as u64, now),
            ServerStatusDto::default()
        );
    }
}
