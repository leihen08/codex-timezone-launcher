#![cfg(windows)]

use chatgpt_timezone_launcher::single_instance::{AcquireResult, acquire_named};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_mutex_name() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    format!(
        r"Local\ChatGPTTimeZoneLauncher-Test-{}-{nonce}",
        std::process::id()
    )
}

#[test]
fn only_one_guard_can_hold_the_same_named_mutex() {
    let name = unique_mutex_name();
    let first = match acquire_named(&name).unwrap() {
        AcquireResult::Acquired(guard) => guard,
        AcquireResult::AlreadyRunning => panic!("unique mutex was unexpectedly already held"),
    };

    assert!(matches!(
        acquire_named(&name).unwrap(),
        AcquireResult::AlreadyRunning
    ));

    drop(first);
    assert!(matches!(
        acquire_named(&name).unwrap(),
        AcquireResult::Acquired(_)
    ));
}

#[test]
fn mutex_name_cannot_be_empty() {
    assert!(acquire_named("").is_err());
}
