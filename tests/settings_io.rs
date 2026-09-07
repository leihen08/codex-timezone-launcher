use chatgpt_timezone_launcher::config::{
    Settings, load_settings_at, save_settings_at, settings_path_from_local_app_data,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "chatgpt-timezone-launcher-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn settings_path_is_fixed_below_local_app_data() {
    let path = settings_path_from_local_app_data(Path::new(r"C:\Users\Test\AppData\Local"));
    assert_eq!(
        path,
        PathBuf::from(r"C:\Users\Test\AppData\Local\ChatGPTTimeZoneLauncher\settings.json")
    );
}

#[test]
fn missing_settings_is_a_first_run_not_an_error() {
    let directory = TestDirectory::new();
    let path = directory.0.join("settings.json");

    assert_eq!(load_settings_at(&path).unwrap(), None);
}

#[test]
fn saved_timezone_is_remembered_and_can_be_replaced() {
    let directory = TestDirectory::new();
    let path = directory.0.join("nested").join("settings.json");

    save_settings_at(&path, &Settings::new("Asia/Shanghai").unwrap()).unwrap();
    assert_eq!(
        load_settings_at(&path).unwrap().unwrap().timezone,
        "Asia/Shanghai"
    );

    save_settings_at(&path, &Settings::new("Europe/Berlin").unwrap()).unwrap();
    assert_eq!(
        load_settings_at(&path).unwrap().unwrap().timezone,
        "Europe/Berlin"
    );
}

#[test]
fn corrupt_settings_is_reported_instead_of_silently_accepted() {
    let directory = TestDirectory::new();
    let path = directory.0.join("settings.json");
    fs::write(&path, b"not json").unwrap();

    assert!(load_settings_at(&path).unwrap_err().contains("设置"));
}
