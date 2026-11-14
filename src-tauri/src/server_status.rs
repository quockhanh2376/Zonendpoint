use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MAX_STATUS_FILE_BYTES: u64 = 64 * 1024; // 64 KB
pub const MAX_SIGNATURES: u64 = 1_000_000_000;
pub const STALE_THRESHOLD: Duration = Duration::from_secs(2 * 3600); // 2 hours

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ServerStatusDto {
    pub signatures: Option<u64>,
    pub loaded_at: Option<String>,
    pub fetched_at: Option<String>,
    pub stale: bool,
}

impl ServerStatusDto {
    pub fn signature_generation(&self) -> Option<String> {
        match (self.signatures, &self.loaded_at) {
            (Some(sigs), Some(loaded)) if sigs > 0 && !loaded.trim().is_empty() => {
                Some(format!("{sigs}:{loaded}"))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawServerStatus {
    signatures: Option<serde_json::Value>,
    #[serde(alias = "loaded_at_utc")]
    loaded_at: Option<String>,
    #[serde(alias = "fetched_at_utc")]
    fetched_at: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    engine: Option<String>,
}

pub fn format_iso8601_vn(epoch_secs: u64) -> String {
    let local_secs = epoch_secs.saturating_add(7 * 3600);
    let sec = (local_secs % 60) as u32;
    let total_mins = local_secs / 60;
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
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+07:00",
        year, month, day, hour, min, sec
    )
}

/// Parses an ISO 8601 timestamp that strictly matches either:
/// 1. `YYYY-MM-DDTHH:MM:SS+07:00` (length 25, Vietnam UTC+7) -> returns absolute UTC epoch secs and original string.
/// 2. `YYYY-MM-DDTHH:MM:SSZ` (length 20, legacy UTC) -> returns absolute UTC epoch secs and normalized `+07:00` string.
///
/// Rejects all other offsets (e.g. +00:00, +08:00), fractional seconds, and missing suffixes.
pub fn parse_iso8601_timestamp(s: &str) -> Option<(u64, String)> {
    let s = s.trim();
    let is_vn = s.ends_with("+07:00");
    let is_utc = s.ends_with('Z');

    if is_vn {
        if s.len() != 25 {
            return None;
        }
    } else if is_utc {
        if s.len() != 20 {
            return None;
        }
    } else {
        return None;
    }

    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    if is_vn
        && (b[19] != b'+'
            || b[20] != b'0'
            || b[21] != b'7'
            || b[22] != b':'
            || b[23] != b'0'
            || b[24] != b'0')
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

    if is_vn {
        // total_secs is in UTC+7; subtract 7 hours to get absolute UTC epoch seconds
        let epoch_secs = total_secs - 7 * 3600;
        if epoch_secs < 0 {
            None
        } else {
            Some((epoch_secs as u64, s.to_string()))
        }
    } else {
        // total_secs is in UTC
        if total_secs < 0 {
            None
        } else {
            let epoch_secs = total_secs as u64;
            Some((epoch_secs, format_iso8601_vn(epoch_secs)))
        }
    }
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

    let loaded_at_raw = match raw.loaded_at {
        Some(s) if !s.trim().is_empty() => s,
        _ => return ServerStatusDto::default(),
    };

    // Strictly validate loaded_at format and retrieve normalized representation
    let (_loaded_epoch, loaded_at_str) = match parse_iso8601_timestamp(&loaded_at_raw) {
        Some(pair) => pair,
        None => return ServerStatusDto::default(),
    };

    // Handle fetched_at (or fallback to file modification time)
    let (fetched_secs, fetched_at_str) = match raw.fetched_at {
        Some(s) if !s.trim().is_empty() => {
            match parse_iso8601_timestamp(&s) {
                Some(pair) => pair,
                None => return ServerStatusDto::default(), // Malformed fetched_at rejects the whole status
            }
        }
        _ => {
            // Fallback to file_mtime if present, otherwise fallback to loaded_at
            if let Some(mtime) = file_mtime {
                let mtime_secs = mtime.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
                (mtime_secs, format_iso8601_vn(mtime_secs))
            } else {
                let (loaded_secs, _) = parse_iso8601_timestamp(&loaded_at_raw).unwrap();
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
        loaded_at: Some(loaded_at_str),
        fetched_at: Some(fetched_at_str),
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
        let json_neg = r#"{"signatures": -5, "loaded_at": "2026-11-14T11:39:43+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json_neg, json_neg.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Float number
        let json_float = r#"{"signatures": 123.45, "loaded_at": "2026-11-14T11:39:43+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json_float, json_float.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Decimal signatures from real report (8616497.5)
        let json_decimal = r#"{"signatures": 8616497.5, "loaded_at": "2026-11-14T11:39:43+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json_decimal, json_decimal.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // String number
        let json_str = r#"{"signatures": "8616464", "loaded_at": "2026-11-14T11:39:43+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json_str, json_str.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Number exceeds 1e9
        let json_overflow =
            r#"{"signatures": 1000000001, "loaded_at": "2026-11-14T11:39:43+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json_overflow, json_overflow.len() as u64, now, None),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_file_too_large_rejected() {
        let now = SystemTime::now();
        let json = r#"{"signatures": 8616464, "loaded_at": "2026-11-14T11:39:43+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json, MAX_STATUS_FILE_BYTES + 1, now, None),
            ServerStatusDto::default()
        );
    }

    #[test]
    fn test_utf8_bom_support() {
        let (fetched_epoch, _) =
            parse_iso8601_timestamp("2026-11-14T11:45:00+07:00").expect("valid iso vn");
        let now = time_from_secs(fetched_epoch + 1800);

        let json_with_bom = "\u{feff}{\"signatures\": 8616464, \"loaded_at\": \"2026-11-14T11:39:43+07:00\", \"fetched_at\": \"2026-11-14T11:45:00+07:00\"}";

        let dto = parse_server_status_content(json_with_bom, json_with_bom.len() as u64, now, None);
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at: Some("2026-11-14T11:39:43+07:00".to_string()),
                fetched_at: Some("2026-11-14T11:45:00+07:00".to_string()),
                stale: false,
            }
        );
    }

    #[test]
    fn test_valid_fresh_signatures_with_plus_seven() {
        let (fetched_epoch, _) =
            parse_iso8601_timestamp("2026-11-14T11:55:00+07:00").expect("valid iso");
        // Test time 30 mins after fetched
        let now = time_from_secs(fetched_epoch + 1800);

        let json = r#"{
            "signatures": 8616464,
            "loaded_at": "2026-11-14T11:39:43+07:00",
            "fetched_at": "2026-11-14T11:55:00+07:00",
            "engine": "ClamAV 1.5.4/28182"
        }"#;

        let dto = parse_server_status_content(json, json.len() as u64, now, None);
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at: Some("2026-11-14T11:39:43+07:00".to_string()),
                fetched_at: Some("2026-11-14T11:55:00+07:00".to_string()),
                stale: false,
            }
        );
    }

    #[test]
    fn test_legacy_z_normalized_to_plus_seven() {
        // Legacy file with UTC 'Z' format and old field names loaded_at_utc / fetched_at_utc
        let (fetched_epoch, _) =
            parse_iso8601_timestamp("2026-11-14T03:55:00Z").expect("valid legacy utc");
        let now = time_from_secs(fetched_epoch + 1800);

        let json = r#"{
            "signatures": 8616464,
            "loaded_at_utc": "2026-11-14T03:39:43Z",
            "fetched_at_utc": "2026-11-14T03:55:00Z",
            "engine": "ClamAV 1.5.4/28182"
        }"#;

        let dto = parse_server_status_content(json, json.len() as u64, now, None);
        // 03:39:43 UTC -> 10:39:43+07:00; 03:55:00 UTC -> 10:55:00+07:00
        assert_eq!(
            dto,
            ServerStatusDto {
                signatures: Some(8616464),
                loaded_at: Some("2026-11-14T10:39:43+07:00".to_string()),
                fetched_at: Some("2026-11-14T10:55:00+07:00".to_string()),
                stale: false,
            }
        );
    }

    #[test]
    fn test_stale_anti_skew_seven_hours() {
        // Verify that +07:00 timestamps are compared against true absolute epoch time,
        // preventing a 7-hour (25,200s) timezone shift from causing false stale flags.
        let (fetched_epoch, _) =
            parse_iso8601_timestamp("2026-11-14T11:45:00+07:00").expect("valid iso vn");

        // Case A: 15 minutes after fetch in real absolute time
        let now_15m = time_from_secs(fetched_epoch + 900);
        let json = r#"{"signatures": 8616497, "loaded_at": "2026-11-14T11:41:49+07:00", "fetched_at": "2026-11-14T11:45:00+07:00"}"#;

        let dto_fresh = parse_server_status_content(json, json.len() as u64, now_15m, None);
        assert!(!dto_fresh.stale, "15m elapsed must NOT be stale");
        assert_eq!(dto_fresh.signatures, Some(8616497));

        // Case B: 2 hours and 1 second after fetch in real absolute time -> stale
        let now_stale = time_from_secs(fetched_epoch + 7201);
        let dto_stale = parse_server_status_content(json, json.len() as u64, now_stale, None);
        assert!(dto_stale.stale, "2h+ elapsed MUST be stale");
    }

    #[test]
    fn test_stale_based_on_fetched_at_not_loaded_at() {
        // loaded_at is 10 hours ago (clamd did not reload because no new signatures)
        let (loaded_epoch, _) =
            parse_iso8601_timestamp("2026-11-14T07:00:00+07:00").expect("valid iso");
        // fetched_at is 30 minutes ago (client fetched recently)
        let fetched_epoch = loaded_epoch + 9 * 3600 + 1800; // 16:30:00+07:00
        let fetched_str = format_iso8601_vn(fetched_epoch);

        // now is 16:45:00+07:00 (15 minutes after fetch) -> MUST NOT be stale!
        let now_fresh = time_from_secs(fetched_epoch + 900);
        let json = format!(
            r#"{{"signatures": 8616497, "loaded_at": "2026-11-14T07:00:00+07:00", "fetched_at": "{}"}}"#,
            fetched_str
        );

        let dto_fresh = parse_server_status_content(&json, json.len() as u64, now_fresh, None);
        assert!(!dto_fresh.stale);
        assert_eq!(dto_fresh.signatures, Some(8616497));

        // now is 2 hours and 1 second after fetch -> MUST be stale!
        let now_stale = time_from_secs(fetched_epoch + 7201);
        let dto_stale = parse_server_status_content(&json, json.len() as u64, now_stale, None);
        assert!(dto_stale.stale);
        assert_eq!(dto_stale.signatures, Some(8616497));
    }

    #[test]
    fn test_legacy_file_without_fetched_at_uses_file_mtime() {
        let (loaded_epoch, _) =
            parse_iso8601_timestamp("2026-11-14T07:00:00+07:00").expect("valid iso");
        let mtime_secs = loaded_epoch + 10 * 3600; // 17:00:00+07:00
        let mtime = time_from_secs(mtime_secs);

        let json = r#"{"signatures": 8616464, "loaded_at": "2026-11-14T07:00:00+07:00"}"#;

        // 30 mins after mtime -> fresh
        let now_fresh = time_from_secs(mtime_secs + 1800);
        let dto_fresh =
            parse_server_status_content(json, json.len() as u64, now_fresh, Some(mtime));
        assert!(!dto_fresh.stale);
        assert_eq!(
            dto_fresh.fetched_at,
            Some("2026-11-14T17:00:00+07:00".to_string())
        );

        // 2 hours and 5 mins after mtime -> stale
        let now_stale = time_from_secs(mtime_secs + 7500);
        let dto_stale =
            parse_server_status_content(json, json.len() as u64, now_stale, Some(mtime));
        assert!(dto_stale.stale);
    }

    #[test]
    fn test_strict_timezone_rejections() {
        let now = SystemTime::now();

        // Timezone offset (+00:00) instead of +07:00 or Z
        let json_zero = r#"{"signatures": 8616464, "loaded_at": "2026-11-14T03:39:43+00:00"}"#;
        assert_eq!(
            parse_server_status_content(json_zero, json_zero.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Timezone offset (+08:00)
        let json_eight = r#"{"signatures": 8616464, "loaded_at": "2026-11-14T11:39:43+08:00"}"#;
        assert_eq!(
            parse_server_status_content(json_eight, json_eight.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Fractional seconds (.123)
        let json_fractional =
            r#"{"signatures": 8616464, "loaded_at": "2026-11-14T11:39:43.123+07:00"}"#;
        assert_eq!(
            parse_server_status_content(json_fractional, json_fractional.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Missing timezone suffix
        let json_no_tz = r#"{"signatures": 8616464, "loaded_at": "2026-11-14T11:39:43"}"#;
        assert_eq!(
            parse_server_status_content(json_no_tz, json_no_tz.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Arbitrary suffix after +07:00
        let json_suffix =
            r#"{"signatures": 8616464, "loaded_at": "2026-11-14T11:39:43+07:00extra"}"#;
        assert_eq!(
            parse_server_status_content(json_suffix, json_suffix.len() as u64, now, None),
            ServerStatusDto::default()
        );

        // Malformed fetched_at
        let json_bad_fetch = r#"{"signatures": 8616464, "loaded_at": "2026-11-14T11:39:43+07:00", "fetched_at": "2026-11-14 11:39:43"}"#;
        assert_eq!(
            parse_server_status_content(json_bad_fetch, json_bad_fetch.len() as u64, now, None),
            ServerStatusDto::default()
        );
    }
}
