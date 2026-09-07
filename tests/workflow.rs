use chatgpt_timezone_launcher::workflow::{SaveLaunchOutcome, save_and_launch_with};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

#[test]
fn running_client_saves_selection_but_never_launches_or_terminates_it() {
    let saved_timezone = RefCell::new(None);
    let launch_called = Cell::new(false);

    let outcome = save_and_launch_with(
        "Asia/Shanghai",
        |settings| {
            *saved_timezone.borrow_mut() = Some(settings.timezone.clone());
            Ok(())
        },
        || Ok(PathBuf::from(r"C:\Apps\ChatGPT.exe")),
        |_| Ok(true),
        |_, _| {
            launch_called.set(true);
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(*saved_timezone.borrow(), Some("Asia/Shanghai".into()));
    assert!(!launch_called.get());
    assert!(matches!(outcome, SaveLaunchOutcome::AlreadyRunning(_)));
}

#[test]
fn stopped_client_is_launched_with_validated_timezone() {
    let launched = RefCell::new(None);

    let outcome = save_and_launch_with(
        "Europe/Paris",
        |_| Ok(()),
        || Ok(PathBuf::from(r"C:\Apps\ChatGPT.exe")),
        |_| Ok(false),
        |path, timezone| {
            *launched.borrow_mut() = Some((path.to_path_buf(), timezone.to_string()));
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        *launched.borrow(),
        Some((PathBuf::from(r"C:\Apps\ChatGPT.exe"), "Europe/Paris".into()))
    );
    assert!(matches!(outcome, SaveLaunchOutcome::Launched(_)));
}

#[test]
fn invalid_timezone_stops_before_any_side_effect() {
    let side_effect = Cell::new(false);

    let result = save_and_launch_with(
        "Asia/Shanghai\nTZ=UTC",
        |_| {
            side_effect.set(true);
            Ok(())
        },
        || {
            side_effect.set(true);
            Ok(PathBuf::new())
        },
        |_| Ok(false),
        |_: &Path, _| Ok(()),
    );

    assert!(result.is_err());
    assert!(!side_effect.get());
}
