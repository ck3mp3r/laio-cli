use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use miette::{Result, miette};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::common::path::home_dir;

const STATE_DIR_ENV: &str = "LAIO_STATE_DIR";
const XDG_STATE_HOME_ENV: &str = "XDG_STATE_HOME";
const APP_DIR: &str = "laio";
const REX_DIR: &str = "rex";
const RECORD_EXTENSION: &str = "json";
const KEY_BYTES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RexSessionRecord {
    pub(crate) rex_session_id: String,
    pub(crate) laio_name: String,
    pub(crate) config_path: String,
    pub(crate) variables: String,
    pub(crate) created_at: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RexSessionStore {
    dir: PathBuf,
}

impl RexSessionStore {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// A record lives at `<dir>/<key>.json`, where `key` is the first 8 lowercase
    /// hex digits of SHA-256 over the verbatim laio session name.
    ///
    /// Two distinct names can share a key. A collision is never resolved
    /// silently: the write refuses to replace a record that belongs to another
    /// name. The start path checks the key before it creates the session; a
    /// collision that appears in the race window between that check and this
    /// write is logged by the caller and the write is skipped, so the stored
    /// record keeps its owner. Concurrent writes to the same name are
    /// last-writer-wins; the atomic temporary file plus rename keeps every record
    /// whole.
    pub(crate) fn write_record(&self, record: &RexSessionRecord) -> Result<()> {
        if let Some(existing) = self.read_key_file(&record_key(&record.laio_name))?
            && existing.laio_name != record.laio_name
        {
            return Err(miette!(
                "Refusing to overwrite the rex session record of '{}' with '{}': both names share the record key",
                existing.laio_name,
                record.laio_name
            ));
        }

        fs::create_dir_all(&self.dir)
            .map_err(|error| miette!("failed to create '{}': {error}", self.dir.display()))?;

        let path = self.key_path(&record_key(&record.laio_name));
        let tmp_path = path.with_extension(format!("{RECORD_EXTENSION}.tmp"));
        let json = serde_json::to_string_pretty(record)
            .map_err(|error| miette!("failed to serialise rex session record: {error}"))?;

        fs::write(&tmp_path, json)
            .map_err(|error| miette!("failed to write '{}': {error}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .map_err(|error| miette!("failed to replace '{}': {error}", path.display()))
    }

    pub(crate) fn read_by_name(&self, laio_name: &str) -> Result<Option<RexSessionRecord>> {
        let Some(record) = self.read_key_file(&record_key(laio_name))? else {
            return Ok(None);
        };

        if record.laio_name != laio_name {
            log::warn!(
                "Ignoring the rex session record '{}': it belongs to another laio name",
                record.laio_name
            );

            return Ok(None);
        }

        Ok(Some(record))
    }

    pub(crate) fn find_by_session_id(
        &self,
        rex_session_id: &str,
    ) -> Result<Option<RexSessionRecord>> {
        Ok(self
            .list()?
            .into_iter()
            .find(|record| record.rex_session_id == rex_session_id))
    }

    pub(crate) fn stored_name_at_key(&self, laio_name: &str) -> Result<Option<String>> {
        Ok(self
            .read_key_file(&record_key(laio_name))?
            .map(|record| record.laio_name))
    }

    pub(crate) fn delete_by_name(&self, laio_name: &str) -> Result<()> {
        if let Some(record) = self.read_key_file(&record_key(laio_name))?
            && record.laio_name != laio_name
        {
            log::warn!(
                "Refusing to delete the rex session record '{}': it belongs to another laio name",
                record.laio_name
            );

            return Ok(());
        }

        delete_file(&self.key_path(&record_key(laio_name)))
    }

    pub(crate) fn prune(&self, live_session_ids: &[String]) -> Result<Vec<String>> {
        let mut pruned = Vec::new();

        for (path, record) in self.scanned_records()? {
            if live_session_ids.contains(&record.rex_session_id) {
                continue;
            }

            delete_file(&path)?;
            pruned.push(record.rex_session_id);
        }

        Ok(pruned)
    }

    pub(crate) fn list(&self) -> Result<Vec<RexSessionRecord>> {
        Ok(self
            .scanned_records()?
            .into_iter()
            .map(|(_, record)| record)
            .collect())
    }

    fn read_key_file(&self, key: &str) -> Result<Option<RexSessionRecord>> {
        let path = self.key_path(key);

        match fs::read_to_string(&path) {
            Ok(json) => Ok(parse_record(&path, &json)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(miette!("failed to read '{}': {error}", path.display())),
        }
    }

    fn scanned_records(&self) -> Result<Vec<(PathBuf, RexSessionRecord)>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(miette!("failed to list '{}': {error}", self.dir.display()));
            }
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(RECORD_EXTENSION))
            .collect();
        paths.sort();

        Ok(paths
            .iter()
            .filter_map(|path| read_record_file(path).map(|record| self.migrate(path, record)))
            .collect())
    }

    fn migrate(&self, path: &Path, record: RexSessionRecord) -> (PathBuf, RexSessionRecord) {
        let key = record_key(&record.laio_name);

        if path.file_stem().and_then(|stem| stem.to_str()) == Some(key.as_str()) {
            return (path.to_path_buf(), record);
        }

        if let Err(error) = self.write_record(&record) {
            log::warn!(
                "Failed to migrate the rex session record '{}': {error}",
                path.display()
            );

            return (path.to_path_buf(), record);
        }

        if let Err(error) = fs::remove_file(path) {
            log::warn!(
                "Failed to remove the legacy rex session record '{}': {error}",
                path.display()
            );
        }

        (self.key_path(&key), record)
    }

    fn key_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.{RECORD_EXTENSION}"))
    }
}

fn record_key(laio_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(laio_name.as_bytes());
    let digest = hasher.finalize();

    digest
        .iter()
        .take(KEY_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn delete_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(miette!("failed to delete '{}': {error}", path.display())),
    }
}

fn read_record_file(path: &Path) -> Option<RexSessionRecord> {
    match fs::read_to_string(path) {
        Ok(json) => parse_record(path, &json),
        Err(error) => {
            log::warn!(
                "Skipping unreadable rex session record '{}': {error}",
                path.display()
            );
            None
        }
    }
}

fn parse_record(path: &Path, json: &str) -> Option<RexSessionRecord> {
    match serde_json::from_str(json) {
        Ok(record) => Some(record),
        Err(error) => {
            log::warn!("Skipping rex session record '{}': {error}", path.display());
            None
        }
    }
}

pub(crate) fn laio_state_dir() -> PathBuf {
    let override_dir = env::var(STATE_DIR_ENV).ok();
    let xdg_state_home = env::var(XDG_STATE_HOME_ENV).ok();
    let home = home_dir().unwrap_or_else(|_| "/".to_string());

    resolve_state_dir(override_dir.as_deref(), xdg_state_home.as_deref(), &home)
}

pub(crate) fn resolve_state_dir(
    override_dir: Option<&str>,
    xdg_state_home: Option<&str>,
    home: &str,
) -> PathBuf {
    match (override_dir, xdg_state_home) {
        (Some(dir), _) => PathBuf::from(dir),
        (None, Some(xdg)) => PathBuf::from(xdg).join(APP_DIR).join(REX_DIR),
        (None, None) => PathBuf::from(home)
            .join(".local")
            .join("state")
            .join(APP_DIR)
            .join(REX_DIR),
    }
}
