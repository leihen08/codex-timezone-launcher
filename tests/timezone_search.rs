use chatgpt_timezone_launcher::timezone::matching_timezones;

fn zones() -> Vec<String> {
    [
        "America/Indiana/Indianapolis",
        "America/New_York",
        "Asia/Shanghai",
        "Etc/UTC",
        "Pacific/Auckland",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[test]
fn search_matches_any_iana_segment_case_insensitively() {
    assert_eq!(
        matching_timezones(&zones(), "shANGhai"),
        vec!["Asia/Shanghai"]
    );
    assert_eq!(
        matching_timezones(&zones(), "new_york"),
        vec!["America/New_York"]
    );
}

#[test]
fn search_prioritizes_prefix_matches_before_inner_matches() {
    let zones = vec!["X/America".to_string(), "America/New_York".to_string()];
    assert_eq!(
        matching_timezones(&zones, "america"),
        vec!["America/New_York", "X/America"]
    );
}

#[test]
fn empty_search_shows_all_zones_and_control_characters_show_none() {
    let zones = zones();
    assert_eq!(matching_timezones(&zones, "  ").len(), zones.len());
    assert!(matching_timezones(&zones, "Asia\nShanghai").is_empty());
}
