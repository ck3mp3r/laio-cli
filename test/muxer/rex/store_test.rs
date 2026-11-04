use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::path::home_dir;
use crate::muxer::rex::store::{
    RexSessionRecord, RexSessionStore, laio_state_dir, resolve_state_dir,
};

type TestResult<T = ()> = core::result::Result<T, Box<dyn std::error::Error>>;

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

fn unique_dir() -> PathBuf {
    let suffix = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());

    env::temp_dir().join(format!(
        "laio-rex-store-{}-{nanos}-{suffix}",
        std::process::id()
    ))
}

fn record(rex_session_id: &str, laio_name: &str) -> RexSessionRecord {
    RexSessionRecord {
        rex_session_id: rex_session_id.to_string(),
        rex_label: format!("{laio_name}-label"),
        laio_name: laio_name.to_string(),
        config_path: format!("/tmp/configs/{laio_name}.yaml"),
        variables: format!("session_name={laio_name}&path=%2Ftmp"),
        created_at: 1_700_000_000,
    }
}

fn store() -> RexSessionStore {
    RexSessionStore::new(unique_dir())
}

#[test]
fn write_then_read_by_name_round_trips_every_field() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    let expected = record("session:1", "project");

    store.write_record(&expected)?;

    let actual = store
        .read_by_name("project")?
        .ok_or("should read back the record")?;

    assert_eq!(actual.rex_session_id, expected.rex_session_id);
    assert_eq!(actual.rex_label, expected.rex_label);
    assert_eq!(actual.laio_name, expected.laio_name);
    assert_eq!(actual.config_path, expected.config_path);
    assert_eq!(actual.variables, expected.variables);
    assert_eq!(actual.created_at, expected.created_at);
    assert!(
        dir.join("1.json").is_file(),
        "record file is named after the uuid"
    );
    Ok(())
}

#[test]
fn read_by_name_returns_none_for_an_unknown_name() -> TestResult {
    let store = store();
    store.write_record(&record("session:1", "project"))?;

    let found = store.read_by_name("missing")?;

    assert!(found.is_none());
    Ok(())
}

#[test]
fn read_by_name_skips_files_that_are_not_valid_json() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("broken.json"), "{ not json")?;
    let store = RexSessionStore::new(dir);

    let found = store.read_by_name("project")?;

    assert!(found.is_none());
    assert!(store.list()?.is_empty());
    Ok(())
}

#[test]
fn read_by_name_picks_the_record_with_the_matching_name() -> TestResult {
    let store = store();
    store.write_record(&record("session:1", "alpha"))?;
    store.write_record(&record("session:2", "beta"))?;

    let alpha = store.read_by_name("alpha")?.ok_or("should read alpha")?;
    let beta = store.read_by_name("beta")?.ok_or("should read beta")?;

    assert_eq!(alpha.rex_session_id, "session:1");
    assert_eq!(beta.rex_session_id, "session:2");
    Ok(())
}

#[test]
fn read_by_session_reads_the_record_file_directly() -> TestResult {
    let store = store();
    store.write_record(&record("session:1", "project"))?;

    let found = store
        .read_by_session("session:1")?
        .ok_or("should read the session record")?;

    assert_eq!(found.laio_name, "project");
    assert!(store.read_by_session("session:2")?.is_none());
    Ok(())
}

#[test]
fn read_by_session_returns_none_for_a_corrupt_record() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("1.json"), "{ not json")?;
    let store = RexSessionStore::new(dir);

    let found = store.read_by_session("session:1")?;

    assert!(found.is_none());
    Ok(())
}

#[test]
fn read_by_session_errors_when_the_record_path_is_unreadable() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(dir.join("1.json"))?;
    let store = RexSessionStore::new(dir);

    let error = store
        .read_by_session("session:1")
        .err()
        .ok_or("reading a directory as a record should fail")?;
    let message = error.to_string();

    assert!(message.contains("1.json"), "{message}");
    Ok(())
}

#[test]
fn delete_by_session_removes_the_record_file() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "project"))?;

    store.delete_by_session("session:1")?;

    assert!(!dir.join("1.json").exists(), "record file should be gone");
    assert!(store.read_by_session("session:1")?.is_none());
    store.delete_by_session("session:1")?;
    Ok(())
}

#[test]
fn prune_deletes_records_absent_from_the_live_ids() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:a", "alpha"))?;
    store.write_record(&record("session:b", "beta"))?;

    let pruned = store.prune(&["session:a".to_string()])?;

    assert_eq!(pruned, ["session:b"]);
    assert!(
        !dir.join("b.json").exists(),
        "stale record file should be gone"
    );
    assert!(dir.join("a.json").is_file(), "live record file should stay");
    Ok(())
}

#[test]
fn list_returns_every_readable_record() -> TestResult {
    let store = store();
    store.write_record(&record("session:1", "alpha"))?;
    store.write_record(&record("session:2", "beta"))?;

    let names: Vec<String> = store
        .list()?
        .into_iter()
        .map(|record| record.laio_name)
        .collect();

    assert_eq!(names, ["alpha", "beta"]);
    Ok(())
}

#[test]
fn reads_tolerate_a_missing_store_directory() -> TestResult {
    let store = store();

    assert!(store.read_by_name("project")?.is_none());
    assert!(store.read_by_session("session:1")?.is_none());
    assert!(store.list()?.is_empty());
    assert!(store.prune(&["session:1".to_string()])?.is_empty());
    store.delete_by_session("session:1")?;
    Ok(())
}

#[test]
fn resolve_state_dir_prefers_the_override() -> TestResult {
    let resolved = resolve_state_dir(Some("/tmp/laio-state"), Some("/tmp/xdg"), "/tmp/home");

    assert_eq!(resolved, PathBuf::from("/tmp/laio-state"));
    Ok(())
}

#[test]
fn resolve_state_dir_uses_xdg_state_home_when_the_override_is_absent() -> TestResult {
    let resolved = resolve_state_dir(None, Some("/tmp/xdg"), "/tmp/home");

    assert_eq!(resolved, PathBuf::from("/tmp/xdg/laio/rex"));
    Ok(())
}

#[test]
fn resolve_state_dir_falls_back_to_the_home_directory() -> TestResult {
    let resolved = resolve_state_dir(None, None, "/tmp/home");

    assert_eq!(resolved, PathBuf::from("/tmp/home/.local/state/laio/rex"));
    Ok(())
}

#[test]
fn laio_state_dir_applies_the_real_env_to_the_resolver() -> TestResult {
    let override_dir = env::var("LAIO_STATE_DIR").ok();
    let xdg_state_home = env::var("XDG_STATE_HOME").ok();
    let expected = resolve_state_dir(
        override_dir.as_deref(),
        xdg_state_home.as_deref(),
        &home_dir()?,
    );

    assert_eq!(laio_state_dir(), expected);
    Ok(())
}
