use std::env;
use std::fs;
use std::sync::{Arc, Mutex};

use crate::common::muxer::Multiplexer;

use super::mux_test::*;

const SESSION_LIST: &str = r#"{"sessions":[{"session_id":"session:1","label":"laio: work"},{"session_id":"session:2","label":"laio: other"},{"session_id":"session:3","label":"other"}]}"#;

fn set_current_session(value: &str) {
    unsafe {
        env::set_var("REX_SESSION", value);
    }
}

#[test]
fn stop_all_kills_and_deletes_every_laio_session_except_the_current_one() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    harness.seed(&record("session:2", "other"))?;
    let store = harness.store();

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .unit(&["kill", "session:2"])
        .build();
    set_current_session("session:1");

    let result = rex.stop(&None, &None, true, true, false);
    unsafe {
        env::remove_var("REX_SESSION");
    }

    result?;
    assert!(store.read_by_session("session:1")?.is_some());
    assert!(store.read_by_session("session:2")?.is_none());
    Ok(())
}

#[test]
fn stop_all_kills_a_live_laio_label_without_a_record() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .unit(&["kill", "session:1"])
        .unit(&["kill", "session:2"])
        .build();

    Ok(rex.stop(&None, &None, true, true, false)?)
}

#[test]
fn stop_other_sweeps_every_other_laio_session() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new()
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .unit(&["kill", "session:1"])
        .unit(&["kill", "session:2"])
        .build();
    set_current_session("session:3");

    let result = rex.stop(&None, &None, true, false, true);
    unsafe {
        env::remove_var("REX_SESSION");
    }

    Ok(result?)
}

#[test]
fn stop_kills_a_recorded_session_by_its_full_id() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    let store = harness.store();

    let rex = harness.unit(&["kill", "session:1"]).build();

    rex.stop(&Some("work".to_string()), &None, true, false, false)?;

    assert!(store.read_by_session("session:1")?.is_none());
    Ok(())
}

#[test]
fn stop_without_a_name_kills_the_current_rex_session() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    let store = harness.store();

    let rex = harness.unit(&["kill", "session:1"]).build();
    set_current_session("session:1");

    let result = rex.stop(&None, &None, true, false, false);
    unsafe {
        env::remove_var("REX_SESSION");
    }

    result?;
    assert!(store.read_by_session("session:1")?.is_none());
    Ok(())
}

#[test]
fn stop_without_a_name_keeps_a_foreign_live_session_alive() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new()
        .json_args(
            &["session", "ls", "--json"],
            r#"{"sessions":[{"session_id":"session:7","label":"personal"}]}"#,
        )
        .build();
    set_current_session("session:7");

    let result = rex.stop(&None, &None, false, false, false);
    unsafe {
        env::remove_var("REX_SESSION");
    }

    result?;
    Ok(())
}

#[test]
fn stop_without_a_name_kills_a_live_laio_label_without_a_record() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new()
        .json_args(
            &["session", "ls", "--json"],
            r#"{"sessions":[{"session_id":"session:1","label":"laio: work"}]}"#,
        )
        .unit(&["kill", "session:1"])
        .build();
    set_current_session("session:1");

    let result = rex.stop(&None, &None, false, false, false);
    unsafe {
        env::remove_var("REX_SESSION");
    }

    result?;
    Ok(())
}

#[test]
fn stop_without_a_name_returns_ok_when_the_current_session_is_gone() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new()
        .json_args(&["session", "ls", "--json"], r#"{"sessions":[]}"#)
        .build();
    set_current_session("session:9");

    let result = rex.stop(&None, &None, false, false, false);
    unsafe {
        env::remove_var("REX_SESSION");
    }

    result?;
    Ok(())
}

#[test]
fn stop_returns_ok_for_a_name_without_a_record_or_a_live_label() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new()
        .resolve("laio: work", "not_found", "")
        .build();

    Ok(rex.stop(&Some("work".to_string()), &None, false, false, false)?)
}

#[test]
fn stop_runs_the_recorded_shutdown_hooks_before_the_kill() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    let config_dir = unique_dir();
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("work.yaml");
    fs::write(
        &config_path,
        "name: work\npath: /tmp\nshutdown:\n  - command: echo\n    args:\n      - bye\nwindows:\n  - name: main\n",
    )?;

    let mut seeded = record("session:1", "work");
    seeded.config_path = config_path.to_string_lossy().to_string();
    seeded.variables = String::new();
    harness.seed(&seeded)?;
    let store = harness.store();

    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let rex = harness
        .hook_recording("echo bye", Arc::clone(&log), "hook")
        .unit_recording(&["kill", "session:1"], Arc::clone(&log), "kill")
        .build();

    rex.stop(&Some("work".to_string()), &None, false, false, false)?;

    assert_eq!(lock(&log).as_slice(), ["hook", "kill"]);
    assert!(store.read_by_session("session:1")?.is_none());
    Ok(())
}

#[test]
fn stop_prefers_the_manager_supplied_session_for_the_hooks() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let rex = harness
        .hook_recording("echo supplied", Arc::clone(&log), "hook")
        .unit(&["kill", "session:1"])
        .build();

    let session = session_of(
        "
name: work
path: /tmp
shutdown:
  - command: echo
    args:
      - supplied
windows:
  - name: main
",
    )?;

    rex.stop(
        &Some("work".to_string()),
        &Some(session),
        false,
        false,
        false,
    )?;

    assert_eq!(lock(&log).as_slice(), ["hook"]);
    Ok(())
}

#[test]
fn stop_skips_the_hooks_when_skip_cmds_is_set() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness.unit(&["kill", "session:1"]).build();

    Ok(rex.stop(&Some("work".to_string()), &None, true, false, false)?)
}

#[test]
fn stop_kills_the_resolved_label_when_the_record_is_corrupt() -> TestResult {
    let _guard = env_guard();
    let harness = Harness::new();
    fs::create_dir_all(harness.store_dir())?;
    fs::write(harness.store_dir().join("1.json"), "{ not json")?;

    let rex = harness
        .resolve("laio: work", "resolved", "session:9")
        .unit(&["kill", "session:9"])
        .build();

    Ok(rex.stop(&Some("work".to_string()), &None, false, false, false)?)
}

#[test]
fn stop_rejects_a_name_together_with_stop_all() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new().build();

    let error = rex
        .stop(&Some("work".to_string()), &None, false, true, false)
        .err()
        .ok_or("mutually exclusive arguments should fail")?;

    assert!(error.to_string().contains("mutually exclusive"), "{error}");
    Ok(())
}

#[test]
fn stop_requires_a_session_name_outside_a_rex_session() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new().build();

    let error = rex
        .stop(&None, &None, false, false, false)
        .err()
        .ok_or("a missing session name should fail")?;

    assert!(
        error.to_string().contains("Specify laio session"),
        "{error}"
    );
    Ok(())
}

#[test]
fn stop_other_bails_outside_a_rex_session() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new().build();

    let error = rex
        .stop(&None, &None, false, false, true)
        .err()
        .ok_or("stop_other outside a rex session should fail")?;

    assert!(
        error.to_string().contains("Not inside a rex session"),
        "{error}"
    );
    Ok(())
}
