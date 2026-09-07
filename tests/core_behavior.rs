use chatgpt_timezone_launcher::{
    config::{Settings, decode_settings, encode_settings},
    discovery::select_first_existing,
    geo::{
        IP_SERVICE_URL, network_error_message, parse_timezone_response, timezone_from_http_response,
    },
    process::should_block_for_process,
    timezone::validate_timezone,
};
use std::path::{Path, PathBuf};

#[test]
fn accepts_canonical_iana_timezone() {
    assert_eq!(
        validate_timezone("America/New_York").unwrap(),
        "America/New_York"
    );
}

#[test]
fn rejects_non_iana_and_control_character_timezone() {
    assert!(validate_timezone("China Standard Time").is_err());
    assert!(validate_timezone("Asia/Shanghai\nOTHER=value").is_err());
}

#[test]
fn settings_json_round_trips_the_remembered_timezone() {
    let settings = Settings::new("Europe/London").unwrap();
    let json = encode_settings(&settings).unwrap();
    let decoded = decode_settings(&json).unwrap();
    let json_text = String::from_utf8(json).unwrap();

    assert_eq!(decoded.timezone, "Europe/London");
    assert!(!json_text.contains("ip"));
}

#[test]
fn invalid_timezone_in_settings_is_rejected() {
    let json = br#"{"timezone":"not/a-zone"}"#;
    assert!(decode_settings(json).is_err());
}

#[test]
fn legacy_settings_remembered_timezone_is_migrated_without_restart_policy() {
    let json = br#"{
        "Mode":"Offset",
        "TimeZoneName":"America/Los_Angeles",
        "UtcOffsetMinutes":420,
        "RestartIfRunning":true
    }"#;

    let decoded = decode_settings(json).unwrap();
    assert_eq!(decoded.timezone, "America/Los_Angeles");
}

#[test]
fn utf8_bom_settings_are_accepted_for_windows_compatibility() {
    let json = b"\xEF\xBB\xBF{\"timezone\":\"Asia/Tokyo\"}";
    assert_eq!(decode_settings(json).unwrap().timezone, "Asia/Tokyo");
}

#[test]
fn discovery_chooses_first_existing_executable_candidate() {
    let candidates = vec![PathBuf::from("old.exe"), PathBuf::from("ChatGPT.exe")];
    let selected = select_first_existing(&candidates, |path| path == Path::new("ChatGPT.exe"));

    assert_eq!(selected, Some(PathBuf::from("ChatGPT.exe")));
}

#[test]
fn running_chatgpt_blocks_launch_but_unrelated_codex_cli_does_not() {
    let target = Path::new(r"C:\Program Files\WindowsApps\OpenAI.Codex\app\ChatGPT.exe");
    assert!(should_block_for_process(target, "ChatGPT.exe", None));
    assert!(!should_block_for_process(
        target,
        "codex.exe",
        Some(Path::new(r"C:\Users\me\.codex\bin\codex.exe")),
    ));
}

#[test]
fn matching_codex_desktop_path_blocks_launch() {
    let target = Path::new(r"C:\Apps\Codex\Codex.exe");
    assert!(should_block_for_process(
        target,
        "codex.exe",
        Some(Path::new(r"c:\apps\codex\CODEX.EXE")),
    ));
}

#[test]
fn ip_service_response_yields_validated_timezone_without_retaining_ip() {
    let body = br#"{"ip":"203.0.113.9","timezone":"Asia/Tokyo","city":"Tokyo"}"#;
    assert_eq!(parse_timezone_response(body).unwrap(), "Asia/Tokyo");
}

#[test]
fn ip_service_error_and_invalid_timezone_are_clear_failures() {
    let service_error = br#"{"error":true,"reason":"RateLimited"}"#;
    let invalid_zone = br#"{"timezone":"internal.invalid"}"#;

    assert!(
        parse_timezone_response(service_error)
            .unwrap_err()
            .contains("服务")
    );
    assert!(
        parse_timezone_response(invalid_zone)
            .unwrap_err()
            .contains("时区")
    );
}

#[test]
fn ip_lookup_uses_one_fixed_https_endpoint() {
    assert_eq!(IP_SERVICE_URL, "https://ipapi.co/json/");
}

#[test]
fn ip_http_failures_do_not_get_parsed_as_success() {
    let body = br#"{"timezone":"Asia/Shanghai"}"#;

    assert_eq!(
        timezone_from_http_response(200, body).unwrap(),
        "Asia/Shanghai"
    );
    assert!(
        timezone_from_http_response(503, body)
            .unwrap_err()
            .contains("HTTP")
    );
}

#[test]
fn ip_timeout_and_offline_errors_have_distinct_actionable_messages() {
    use windows_sys::Win32::Networking::WinHttp::{
        ERROR_WINHTTP_CANNOT_CONNECT, ERROR_WINHTTP_TIMEOUT,
    };

    assert!(network_error_message(ERROR_WINHTTP_TIMEOUT, "接收").contains("超时"));
    assert!(network_error_message(ERROR_WINHTTP_CANNOT_CONNECT, "连接").contains("检查网络"));
}
