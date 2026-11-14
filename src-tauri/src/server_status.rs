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
    pub fetched_at_utc: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Deserialize)]
struct RawServerStatus {
    signatures: Option<serde_json::Value>,
    loaded_at_utc: Option<String>,
    fetched_at_utc: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    engine: Option<String>,
}

pub fn parse_iso8601_utc(s: &str) -> Option<u64> {
    let s = s.trim();
    // Strictly require exact 20 characters: YYYY-MM-DDTHH:MM:SSZ
    if s.len() != 20 {
        return None;
    }
    let b = s.as_bytes();
    if b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    for &i in &[0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !b[i].is_ascii_digit() {
            return None;
        }
    }

    let year: i64 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let minute: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;

    if !(1970..=2200).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || sec > 60
    // Allow leap second 60
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

pub fn format_iso8601_utc(secs: u64) -> String {
    let sec = (secs % 60) as u32;
    let total_mins = secs / 60;
    let min = (total_mins % 60) as u32;
    let total_hours = total_mins / 60;
    let hour = (total_hours % 24) as u32;
    let mut days = (total_hours / 24) as i64;

    let mut year = 1970;
    loop {
        let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            366
        } else {
            365
        };
        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &d in &month_days {
        if days >= d {
            days -= d;
            month += 1;
        } else {
            break;
        }
    }
    let day = (days + 1) as u32;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

pub fn parse_server_status_content(
    content: &str,
    file_size: u64,
    now: SystemTime,
    file_mtime: Option<SystemTime>,
) -> ServerStatusDto {
    if file_size > MAX_STATUS_FILE_BYTES || content.len() as u64 > MAX_STATUS_FILE_BYTES {
        return ServerStatusDto::default();
    }

    // Strip UTF-8 BOM if present
    let content = content.trim_start_matches('\u{feff}');

    let raw: RawServerStatus = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return ServerStatusDto::default(),
    };

    let sig_val = match raw.signatures {
        Some(serde_json::Value::Number(ref n)) if n.is_u64() => {
            let u = n.as_u64().unwrap();
            if u <= MAX_SIGNATURES {
                u
            } else {
                return ServerStatusDto::default();
            }
        }
        _ => return ServerStatusDto::default(),
    };

    let loaded_at_str = match raw.loaded_at_utc {
        Some(s) if !s.trim().is_empty() => s,
        _ => return ServerStatusDto::default(),
    };

    // Strictly validate loaded_at_utc format
    if parse_iso8601_utc(&loaded_at_str).is_none() {
        return ServerStatusDto::default();
    }

    // Handle fetched_at_utc (or fallback to file modification time)
    let (fetched_secs, fetched_at_str) = match raw.fetched_at_utc {
        Some(s) if !s.trim().is_empty() => {
            match parse_iso8601_utc(&s) {
                Some(secs) => (secs, s),
                None => return ServerStatusDto::default(), // Malformed fetched_at_utc rejects the whole status
            }
        }
        _ => {
            // Fallback to file_mtime if present, otherwise fallback to loaded_at
            if let Some(mtime) = file_mtime {
                let mtime_secs = mtime.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
                (mtime_secs, format_iso8601_utc(mtime_secs))
            } else {
                let loaded_secs = parse_iso8601_utc(&loaded_at_str).unwrap_or(0);
                (loaded_secs, loaded_at_str.clone())
            }
        }
    };

    let now_secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let stale = if now_secs >= fetched_secs {
        (now_secs - fetched_secs) >= STALE_THRESHOLD.as_secs()
    } else {
        // Skewed into future by > 300 seconds
        (fetched_secs - now_secs) > 300
    };

    ServerStatusDto {
        signatures: Some(sig_val),
        loaded_at_utc: Some(loaded_at_str),
        fetched_at_utc: Some(fetched_at_str),
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

    let file_mtime = metadata.modified().ok();
    parse_server_status_content(&content, metadata.len(), now, file_mtime)
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
            parse_server_status_content("", 0, now, None),
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
            parse_server_status_content("{ invalid json", 14, now, None),
            ServerStatusDto::default()
        );
        assert_eq!(
            parse_server_status_content("null", 4, now, None),
            ServerStatusDto::default()
        );
        assert_eq!(
            parse_server_status_content("[]", 2, now, None),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_negative_or_invalid_signatures_number() {
        let now = SystemTime::now();
        // Negative number
        let json_neg = r#"{"signatures": -5, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_neg, json_neg.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Float number
        let json_float = r#"{"signatures": 123.45, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_float, json_float.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Decimal signatures from real report (8616497.5)
        let json_decimal = r#"{"signatures": 8616497.5, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_decimal, json_decimal.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // String number
        let json_str = r#"{"signatures": "8616464", "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_str, json_str.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Number exceeds 1e9
        let json_overflow =
            r#"{"signatures": 1000000001, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json_overflow, json_overflow.len() as u64, now, None),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_file_too_large_rejected() {
        let now = SystemTime::now();
        let json = r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43Z"}"#;
        assert_eq!(
            parse_server_status_content(json, MAX_STATUS_FILE_BYTES + 1, now, None),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_utf8_bom_support() {
        let loaded_secs = parse_iso8601_utc("2026-11-14T03:39:43Z").expect("valid iso");
        let now = time_from_secs(loaded_secs + 1800);

        let json_with_bom = "\u{feff}{\"signatures\": 8616464, \"loaded_at_utc\": \"2026-11-14T03:39:43Z\", \"fetched_at_utc\": \"2026-11-14T03:45:00Z\"}";

        let dto = parse_server_status_content(json_with_bom, json_with_bom.len() as u64, now, None);
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at_utc: Some("2026-11-14T03:39:43Z".to_string()),
                fetched_at_utc: Some("2026-11-14T03:45:00Z".to_string()),
                stale: false,
            }
        );
    }

    #[test]
    fn test_valid_fresh_signatures_with_fetched_at() {
        let _loaded_secs = parse_iso8601_utc("2026-11-14T03:39:43Z").expect("valid iso");
        let fetched_secs = parse_iso8601_utc("2026-11-14T03:55:00Z").expect("valid iso");
        // Test time 30 mins after fetched
        let now = time_from_secs(fetched_secs + 1800);

        let json = r#"{
            "signatures": 8616464,
            "loaded_at_utc": "2026-11-14T03:39:43Z",
            "fetched_at_utc": "2026-11-14T03:55:00Z",
            "engine": "ClamAV 1.5.4/28182"
        }"#;

        let dto = parse_server_status_content(json, json.len() as u64, now, None);
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at_utc: Some("2026-11-14T03:39:43Z".to_string()),
                fetched_at_utc: Some("2026-11-14T03:55:00Z".to_string()),
                stale: false,
            }
        );
    }

    #[test]
    fn test_stale_based_on_fetched_at_not_loaded_at() {
        // loaded_at is 10 hours ago (clamd did not reload because no new signatures)
        let loaded_secs = parse_iso8601_utc("2026-11-14T00:00:00Z").expect("valid iso");
        // fetched_at is 30 minutes ago (client fetched recently)
        let fetched_secs = loaded_secs + 9 * 3600 + 1800; // 09:30:00Z
        let fetched_str = format_iso8601_utc(fetched_secs);

        // now is 09:45:00Z (15 minutes after fetch) -> MUST NOT be stale!
        let now_fresh = time_from_secs(fetched_secs + 900);
        let json = format!(
            r#"{{"signatures": 8616497, "loaded_at_utc": "2026-11-14T00:00:00Z", "fetched_at_utc": "{}"}}"#,
            fetched_str
        );

        let dto_fresh = parse_server_status_content(&json, json.len() as u64, now_fresh, None);
        assert!(!dto_fresh.stale);
        assert_eq!(dto_fresh.signatures, Some(8616497));

        // now is 2 hours and 1 second after fetch -> MUST be stale!
        let now_stale = time_from_secs(fetched_secs + 7201);
        let dto_stale = parse_server_status_content(&json, json.len() as u64, now_stale, None);
        assert!(dto_stale.stale);
        assert_eq!(dto_stale.signatures, Some(8616497));
    }

    #[test]
    fn test_legacy_file_without_fetched_at_uses_file_mtime() {
        let loaded_secs = parse_iso8601_utc("2026-11-14T00:00:00Z").expect("valid iso");
        let mtime_secs = loaded_secs + 10 * 3600; // 10:00:00Z
        let mtime = time_from_secs(mtime_secs);

        let json = r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T00:00:00Z"}"#;

        // 30 mins after mtime -> fresh
        let now_fresh = time_from_secs(mtime_secs + 1800);
        let dto_fresh =
            parse_server_status_content(json, json.len() as u64, now_fresh, Some(mtime));
        assert!(!dto_fresh.stale);
        assert_eq!(
            dto_fresh.fetched_at_utc,
            Some("2026-11-14T10:00:00Z".to_string())
        );

        // 2 hours and 5 mins after mtime -> stale
        let now_stale = time_from_secs(mtime_secs + 7500);
        let dto_stale =
            parse_server_status_content(json, json.len() as u64, now_stale, Some(mtime));
        assert!(dto_stale.stale);
    }

    #[test]
    fn test_strict_iso8601_utc_rejections() {
        let now = SystemTime::now();

        // Fractional seconds (.123)
        let json_fractional =
            r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43.123Z"}"#;
        assert_eq!(
            parse_server_status_content(json_fractional, json_fractional.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Timezone offset (+00:00) instead of Z
        let json_offset =
            r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43+00:00"}"#;
        assert_eq!(
            parse_server_status_content(json_offset, json_offset.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Missing Z suffix
        let json_no_z = r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43"}"#;
        assert_eq!(
            parse_server_status_content(json_no_z, json_no_z.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Arbitrary suffix after seconds
        let json_suffix =
            r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43Zextra"}"#;
        assert_eq!(
            parse_server_status_content(json_suffix, json_suffix.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Malformed fetched_at_utc
        let json_bad_fetch = r#"{"signatures": 8616464, "loaded_at_utc": "2026-11-14T03:39:43Z", "fetched_at_utc": "2026-11-14 03:39:43"}"#;
        assert_eq!(
            parse_server_status_content(json_bad_fetch, json_bad_fetch.len() as u64, now, None),
            ServerStatusDto::default()
        );
    }
}
