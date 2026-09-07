use crate::config::{Settings, save_settings};
use crate::discovery::discover_client;
use crate::process::{is_client_running, launch_client};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveLaunchOutcome {
    AlreadyRunning(PathBuf),
    Launched(PathBuf),
}

pub fn save_and_launch(timezone: &str) -> Result<SaveLaunchOutcome, String> {
    save_and_launch_with(
        timezone,
        save_settings,
        discover_client,
        is_client_running,
        |path, timezone| launch_client(path, timezone).map(|_| ()),
    )
}

pub fn save_and_launch_with<Save, Discover, Running, Launch>(
    timezone: &str,
    save: Save,
    discover: Discover,
    running: Running,
    launch: Launch,
) -> Result<SaveLaunchOutcome, String>
where
    Save: FnOnce(&Settings) -> Result<(), String>,
    Discover: FnOnce() -> Result<PathBuf, String>,
    Running: FnOnce(&Path) -> Result<bool, String>,
    Launch: FnOnce(&Path, &str) -> Result<(), String>,
{
    let settings = Settings::new(timezone)?;
    save(&settings)?;

    let executable = discover()?;
    if running(&executable)? {
        return Ok(SaveLaunchOutcome::AlreadyRunning(executable));
    }
    launch(&executable, &settings.timezone)?;
    Ok(SaveLaunchOutcome::Launched(executable))
}
