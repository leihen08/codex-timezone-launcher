#![cfg(windows)]

use chatgpt_timezone_launcher::{
    discovery::{ClientTarget, discover_client_target},
    process::launch_client_target_with,
    store_launch::{
        RESUME_EVENT_PREFIX, build_debugger_command_line, build_timezone_environment,
        parse_resume_request, resume_package_thread,
    },
};
use std::ffi::OsString;
use std::path::Path;
use std::{cell::Cell, path::PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentThreadId, WaitForSingleObject,
};

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

#[test]
fn store_target_dispatches_to_package_activation_not_direct_create_process() {
    let direct_launch_called = Cell::new(false);
    let package_activation_called = Cell::new(false);
    let target = ClientTarget::StorePackage {
        executable: PathBuf::from(r"C:\WindowsApps\OpenAI.Codex\app\ChatGPT.exe"),
        package_full_name: "OpenAI.Codex_1.0.0.0_x64__publisher".into(),
        app_user_model_id: "OpenAI.Codex_publisher!App".into(),
    };

    launch_client_target_with(
        &target,
        "Asia/Shanghai",
        |_, _| {
            direct_launch_called.set(true);
            Ok(())
        },
        |_, _, _| {
            package_activation_called.set(true);
            Ok(())
        },
    )
    .unwrap();

    assert!(!direct_launch_called.get());
    assert!(package_activation_called.get());
}

#[test]
fn resumer_opens_launcher_event_and_signals_after_thread_access() {
    let thread_id = unsafe { GetCurrentThreadId() };
    let event_name = format!(
        "{RESUME_EVENT_PREFIX}test-{}-{thread_id}",
        std::process::id()
    );
    let event_name_wide: Vec<u16> = event_name.encode_utf16().chain(Some(0)).collect();
    let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, event_name_wide.as_ptr()) };
    assert!(!event.is_null());

    let request = chatgpt_timezone_launcher::store_launch::ResumeRequest {
        thread_id,
        event_name,
    };
    resume_package_thread(&request).unwrap();
    assert_eq!(unsafe { WaitForSingleObject(event, 0) }, WAIT_OBJECT_0);
    unsafe { CloseHandle(event) };
}
