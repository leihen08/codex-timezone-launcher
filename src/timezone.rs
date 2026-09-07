use chrono_tz::Tz;
use std::str::FromStr;

pub const DEFAULT_TIMEZONE: &str = "Etc/UTC";

pub fn validate_timezone(value: &str) -> Result<String, String> {
    if value.is_empty() || value.len() > 64 || value.trim() != value {
        return Err("请选择有效的 IANA 时区。".into());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '+'))
    {
        return Err("时区名称包含不允许的字符。".into());
    }

    Tz::from_str(value)
        .map(|timezone| timezone.name().to_string())
        .map_err(|_| "无法识别该 IANA 时区。".into())
}

pub fn all_timezones() -> Vec<String> {
    let mut zones: Vec<_> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|timezone| timezone.name().to_string())
        .collect();
    zones.sort_unstable();
    zones
}

pub fn matching_timezones<'a>(zones: &'a [String], query: &str) -> Vec<&'a str> {
    let query = query.trim();
    if query.len() > 64 || query.chars().any(char::is_control) {
        return Vec::new();
    }
    if query.is_empty() {
        return zones.iter().map(String::as_str).collect();
    }

    let query = query.to_ascii_lowercase();
    let mut prefix_matches = Vec::new();
    let mut inner_matches = Vec::new();
    for zone in zones {
        let normalized = zone.to_ascii_lowercase();
        if normalized.starts_with(&query) {
            prefix_matches.push(zone.as_str());
        } else if normalized.contains(&query) {
            inner_matches.push(zone.as_str());
        }
    }
    prefix_matches.extend(inner_matches);
    prefix_matches
}
