#![cfg(windows)]

use chatgpt_timezone_launcher::{
    discovery::{ClientTarget, discover_client_target},
    store_launch::{
        RESUME_EVENT_PREFIX, build_debugger_command_line, build_timezone_environment,
        parse_resume_request,
    },
};
use std::ffi::OsString;
use std::path::Path;

#[test]
fn installed_store_client_has_activation_metadata_not_just_internal_exe_path() {
    let target =
        discover_client_target().expect("Codex Store package is installed on this machine");

    match target {
        ClientTarget::StorePackage {
            executable,
            package_full_name,
            app_user_model_id,
        } => {
            assert!(executable.ends_with(r"app\ChatGPT.exe"));
            assert!(package_full_name.starts_with("OpenAI.Codex_"));
            assert_eq!(app_user_model_id, "OpenAI.Codex_2p2nqsd0c76g0!App");
        }
        ClientTarget::DesktopExecutable(_) => panic!("Store install was misclassified as desktop"),
    }
}

#[test]
fn package_debugger_command_quotes_executable_and_uses_private_event_namespace() {
    let event_name = format!("{RESUME_EVENT_PREFIX}123-456");
    let command = build_debugger_command_line(
        Path::new(r"C:\Program Files\Launcher\launcher.exe"),
        &event_name,
    )
    .unwrap();

    assert_eq!(
        command,
        format!("\"C:\\Program Files\\Launcher\\launcher.exe\" --resume-event {event_name}")
    );
}

#[test]
fn resumer_accepts_only_numeric_thread_and_launcher_owned_event() {
    let event_name = format!("{RESUME_EVENT_PREFIX}123-456");
    let args = [
        OsString::from("launcher.exe"),
        OsString::from("--resume-event"),
        OsString::from(&event_name),
        OsString::from("-p"),
        OsString::from("900"),
        OsString::from("-tid"),
        OsString::from("901"),
    ];

    let request = parse_resume_request(args).unwrap();
    assert_eq!(request.thread_id, 901);
    assert_eq!(request.event_name, event_name);

    let invalid_event = [
        OsString::from("launcher.exe"),
        OsString::from("--resume-event"),
        OsString::from(r"Global\OtherApp"),
        OsString::from("-tid"),
        OsString::from("901"),
    ];
    assert!(parse_resume_request(invalid_event).is_none());
}

#[test]
fn package_environment_is_double_null_terminated_and_contains_only_timezone() {
    let environment = build_timezone_environment("Asia/Shanghai").unwrap();
    let expected: Vec<u16> = "TZ=Asia/Shanghai\0\0".encode_utf16().collect();

    assert_eq!(environment, expected);
}
