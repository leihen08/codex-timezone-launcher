#![cfg(windows)]

use chatgpt_timezone_launcher::{
    discovery::{candidate_paths_from_roots, discover_client, package_executable_candidates},
    process::{build_launch_command, is_client_running},
};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[test]
fn conventional_install_candidates_cover_per_user_and_machine_locations() {
    let paths = candidate_paths_from_roots(Path::new(r"C:\Local"), Path::new(r"C:\Programs"));

    assert!(paths.contains(&PathBuf::from(r"C:\Local\Programs\ChatGPT\ChatGPT.exe")));
    assert!(paths.contains(&PathBuf::from(r"C:\Programs\Codex\Codex.exe")));
}

#[test]
fn store_package_root_maps_to_known_desktop_executable_locations() {
    let paths = package_executable_candidates(Path::new(r"C:\WindowsApps\OpenAI.Codex_1"));

    assert_eq!(
        paths.first().unwrap(),
        &PathBuf::from(r"C:\WindowsApps\OpenAI.Codex_1\app\ChatGPT.exe")
    );
    assert!(paths.contains(&PathBuf::from(r"C:\WindowsApps\OpenAI.Codex_1\Codex.exe")));
}

#[test]
fn launch_command_has_no_shell_and_sets_timezone_only_on_the_child() {
    let original = std::env::var_os("TZ");
    let command = build_launch_command(Path::new(r"C:\Apps\ChatGPT.exe"), "Asia/Tokyo").unwrap();

    assert_eq!(command.get_program(), OsStr::new(r"C:\Apps\ChatGPT.exe"));
    assert_eq!(command.get_args().count(), 0);
    assert!(command.get_envs().any(|(key, value)| {
        key == OsStr::new("TZ") && value == Some(OsStr::new("Asia/Tokyo"))
    }));
    assert_eq!(std::env::var_os("TZ"), original);
}

#[test]
fn installed_codex_client_is_discovered_and_current_instance_is_detected() {
    let executable =
        discover_client().expect("this machine has the Codex desktop client installed");
    let filename = executable.file_name().unwrap().to_string_lossy();

    assert!(executable.is_file());
    assert!(
        filename.eq_ignore_ascii_case("ChatGPT.exe") || filename.eq_ignore_ascii_case("Codex.exe")
    );
    assert!(is_client_running(&executable).unwrap());
}
