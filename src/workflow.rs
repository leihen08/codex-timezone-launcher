use crate::config::{Settings, save_settings};
use crate::discovery::{ClientTarget, discover_client_target};
use crate::process::{is_client_running, launch_client_target};
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
        discover_client_target,
        is_client_running,
        launch_client_target,
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
    Discover: FnOnce() -> Result<ClientTarget, String>,
    Running: FnOnce(&Path) -> Result<bool, String>,
    Launch: FnOnce(&ClientTarget, &str) -> Result<(), String>,
{
    let settings = Settings::new(timezone)?;
    save(&settings)?;

    let target = discover()?;
    let executable = target.executable().to_path_buf();
    if running(&executable)? {
        return Ok(SaveLaunchOutcome::AlreadyRunning(executable));
    }
    launch(&target, &settings.timezone)?;
    Ok(SaveLaunchOutcome::Launched(executable))
}
