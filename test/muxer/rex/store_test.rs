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

const PROJECT_KEY: &str = "244210e4";
const FOO_BAR_KEY: &str = "55282c18";
const UEBER_FOO_KEY: &str = "1fb15279";
const ALPHA_KEY: &str = "8ed3f6ad";
const BETA_KEY: &str = "f44e64e7";
const PROBE_1803_KEY: &str = "a22cdbcd";

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
    assert_eq!(actual.laio_name, expected.laio_name);
    assert_eq!(actual.config_path, expected.config_path);
    assert_eq!(actual.variables, expected.variables);
    assert_eq!(actual.created_at, expected.created_at);

    let files: Vec<String> = fs::read_dir(&dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(files, [format!("{PROJECT_KEY}.json")]);
    assert!(
        PROJECT_KEY
            .chars()
            .all(|ch| matches!(ch, '0'..='9' | 'a'..='f')),
        "the record key is 8 lowercase hex digits"
    );
    assert_eq!(PROJECT_KEY.len(), 8);
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
fn read_by_name_returns_none_for_a_corrupt_record() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{PROJECT_KEY}.json")), "{ not json")?;
    let store = RexSessionStore::new(dir);

    let found = store.read_by_name("project")?;

    assert!(found.is_none());
    Ok(())
}

#[test]
fn the_scan_warns_and_skips_a_corrupt_record_file() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("broken.json"), "{ not json")?;
    let store = RexSessionStore::new(dir);

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
fn read_by_name_errors_when_the_record_path_is_unreadable() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(dir.join(format!("{PROJECT_KEY}.json")))?;
    let store = RexSessionStore::new(dir);

    let error = store
        .read_by_name("project")
        .err()
        .ok_or("reading a directory as a record should fail")?;
    let message = error.to_string();

    assert!(message.contains(PROJECT_KEY), "{message}");
    Ok(())
}

#[test]
fn delete_by_name_removes_the_record_file() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "project"))?;

    store.delete_by_name("project")?;

    assert!(
        !dir.join(format!("{PROJECT_KEY}.json")).exists(),
        "record file should be gone"
    );
    assert!(store.read_by_name("project")?.is_none());
    store.delete_by_name("project")?;
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
        !dir.join(format!("{BETA_KEY}.json")).exists(),
        "stale record file should be gone"
    );
    assert!(
        dir.join(format!("{ALPHA_KEY}.json")).is_file(),
        "live record file should stay"
    );
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
    assert!(store.list()?.is_empty());
    assert!(store.prune(&["session:1".to_string()])?.is_empty());
    store.delete_by_name("project")?;
    Ok(())
}

#[test]
fn write_record_uses_one_key_for_a_repeated_name_and_the_last_write_wins() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());

    store.write_record(&record("session:1", "Foo Bar"))?;
    store.write_record(&record("session:2", "Foo Bar"))?;

    let files: Vec<String> = fs::read_dir(&dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(files, [format!("{FOO_BAR_KEY}.json")]);

    let found = store
        .read_by_name("Foo Bar")?
        .ok_or("should read the record back")?;

    assert_eq!(found.laio_name, "Foo Bar");
    assert_eq!(found.rex_session_id, "session:2");
    Ok(())
}

#[test]
fn write_record_keys_a_unicode_name_and_keeps_it_verbatim() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    let written = record("session:1", "Über-Foo");

    store.write_record(&written)?;

    assert!(dir.join(format!("{UEBER_FOO_KEY}.json")).is_file());

    let found = store
        .read_by_name("Über-Foo")?
        .ok_or("should read the unicode record back")?;

    assert_eq!(found, written);
    Ok(())
}

#[test]
fn write_record_refuses_two_names_that_share_a_key() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "probe-1803"))?;

    let error = store
        .write_record(&record("session:2", "probe-89065"))
        .err()
        .ok_or("a record key collision should fail the write")?;
    let message = error.to_string();

    assert!(message.contains("probe-1803"), "{message}");
    assert!(message.contains("probe-89065"), "{message}");
    assert!(dir.join(format!("{PROBE_1803_KEY}.json")).is_file());

    let stored = store
        .read_by_name("probe-1803")?
        .ok_or("the stored record should keep its owner")?;

    assert_eq!(stored.rex_session_id, "session:1");
    Ok(())
}

#[test]
fn the_scan_rekeys_a_legacy_uuid_record_to_the_sha_file() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    let legacy = record("session:1", "project");
    fs::write(dir.join("1.json"), serde_json::to_string_pretty(&legacy)?)?;
    let store = RexSessionStore::new(dir.clone());

    assert!(
        store.read_by_name("project")?.is_none(),
        "a direct read does not scan"
    );
    assert_eq!(store.list()?, vec![legacy.clone()]);

    assert!(!dir.join("1.json").exists(), "the legacy file is gone");
    assert!(dir.join(format!("{PROJECT_KEY}.json")).is_file());
    assert_eq!(store.read_by_name("project")?, Some(legacy));
    Ok(())
}

#[test]
fn the_scan_leaves_a_corrupt_legacy_record_alone() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("1.json"), "{ not json")?;
    let store = RexSessionStore::new(dir.clone());

    assert!(store.list()?.is_empty());
    assert!(
        dir.join("1.json").is_file(),
        "an unreadable legacy file is not touched"
    );
    Ok(())
}

#[test]
fn find_by_session_id_scans_the_records_for_an_exact_id() -> TestResult {
    let store = store();
    store.write_record(&record("session:1", "alpha"))?;
    store.write_record(&record("session:2", "beta"))?;

    assert_eq!(
        store
            .find_by_session_id("session:2")?
            .map(|record| record.laio_name),
        Some("beta".to_string())
    );
    assert!(store.find_by_session_id("session:3")?.is_none());
    Ok(())
}

#[test]
fn read_by_name_ignores_a_record_owned_by_another_name() -> TestResult {
    let store = store();
    store.write_record(&record("session:1", "probe-1803"))?;

    assert!(
        store.read_by_name("probe-89065")?.is_none(),
        "a foreign record must not be served"
    );
    assert_eq!(
        store
            .read_by_name("probe-1803")?
            .map(|record| record.rex_session_id),
        Some("session:1".to_string())
    );
    Ok(())
}

#[test]
fn delete_by_name_refuses_a_record_owned_by_another_name() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "probe-1803"))?;

    store.delete_by_name("probe-89065")?;

    assert!(
        dir.join(format!("{PROBE_1803_KEY}.json")).is_file(),
        "the record of another name must survive"
    );
    assert!(store.read_by_name("probe-1803")?.is_some());
    Ok(())
}

#[test]
fn prune_deletes_only_the_scanned_path_of_a_stale_record() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "probe-1803"))?;
    fs::write(
        dir.join("1.json"),
        serde_json::to_string_pretty(&record("session:2", "probe-89065"))?,
    )?;

    let pruned = store.prune(&["session:1".to_string()])?;

    assert_eq!(pruned, ["session:2"]);
    assert!(
        !dir.join("1.json").exists(),
        "the scanned legacy path is deleted"
    );
    assert!(
        dir.join(format!("{PROBE_1803_KEY}.json")).is_file(),
        "a re-hash would delete the live record of the other name"
    );
    Ok(())
}

#[test]
fn the_scan_keeps_a_legacy_record_whose_key_is_occupied() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "probe-1803"))?;
    fs::write(
        dir.join("1.json"),
        serde_json::to_string_pretty(&record("session:2", "probe-89065"))?,
    )?;

    assert_eq!(store.list()?.len(), 2, "both records stay readable");
    assert!(
        dir.join("1.json").is_file(),
        "the colliding legacy file is kept"
    );
    assert_eq!(
        store
            .read_by_name("probe-1803")?
            .map(|record| record.rex_session_id),
        Some("session:1".to_string())
    );
    Ok(())
}

#[test]
fn the_scan_rekeys_a_marker_era_record_and_drops_its_label() -> TestResult {
    let dir = unique_dir();
    fs::create_dir_all(&dir)?;
    let mut legacy = serde_json::to_value(record("session:1", "project"))?;
    legacy["rex_label"] = serde_json::json!("laio: project");
    fs::write(dir.join("1.json"), serde_json::to_string_pretty(&legacy)?)?;
    let store = RexSessionStore::new(dir.clone());

    assert_eq!(store.list()?.len(), 1);

    let text = fs::read_to_string(dir.join(format!("{PROJECT_KEY}.json")))?;

    assert!(!text.contains("rex_label"), "{text}");
    assert!(!text.contains("laio: project"), "{text}");
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
