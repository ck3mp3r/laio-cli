use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::muxer::rex::store::{RexSessionRecord, RexSessionStore};

use super::{TestResult, record, unique_dir};

#[test]
fn the_scan_migrates_once_and_a_second_scan_changes_nothing() -> TestResult {
    let dir = unique_dir();
    let store = RexSessionStore::new(dir.clone());
    store.write_record(&record("session:1", "alpha"))?;
    let beta = serde_json::to_string_pretty(&record("session:2", "beta"))?;
    fs::write(dir.join("1.json"), beta)?;
    let mut legacy = serde_json::to_value(record("session:3", "Über-Foo"))?;
    legacy["rex_label"] = serde_json::json!("laio: Über-Foo");
    fs::write(dir.join("2.json"), serde_json::to_string_pretty(&legacy)?)?;

    let first = sorted_records(store.list()?);
    let migrated = snapshot(&dir)?;
    let second = sorted_records(store.list()?);

    assert_eq!(first, second);
    assert_eq!(snapshot(&dir)?, migrated);
    Ok(())
}

fn sorted_records(mut records: Vec<RexSessionRecord>) -> Vec<RexSessionRecord> {
    records.sort_by(|left, right| left.laio_name.cmp(&right.laio_name));
    records
}

fn snapshot(dir: &Path) -> TestResult<Vec<String>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata()?;
        let mtime = meta.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();
        files.push(format!("{name}@{mtime}"));
    }

    files.sort();
    Ok(files)
}
