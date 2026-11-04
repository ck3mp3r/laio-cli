use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use miette::{Result, miette};
use serde::{Deserialize, Serialize};

use crate::common::path::home_dir;

const STATE_DIR_ENV: &str = "LAIO_STATE_DIR";
const XDG_STATE_HOME_ENV: &str = "XDG_STATE_HOME";
const APP_DIR: &str = "laio";
const REX_DIR: &str = "rex";
const SESSION_PREFIX: &str = "session:";
const RECORD_EXTENSION: &str = "json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RexSessionRecord {
    pub(crate) rex_session_id: String,
    pub(crate) rex_label: String,
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

    pub(crate) fn write_record(&self, record: &RexSessionRecord) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .map_err(|error| miette!("failed to create '{}': {error}", self.dir.display()))?;

        let path = self.record_path(&record.rex_session_id);
        let tmp_path = path.with_extension(format!("{RECORD_EXTENSION}.tmp"));
        let json = serde_json::to_string_pretty(record)
            .map_err(|error| miette!("failed to serialise rex session record: {error}"))?;

        fs::write(&tmp_path, json)
            .map_err(|error| miette!("failed to write '{}': {error}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .map_err(|error| miette!("failed to replace '{}': {error}", path.display()))
    }

    pub(crate) fn read_by_name(&self, laio_name: &str) -> Result<Option<RexSessionRecord>> {
        Ok(self
            .records()?
            .into_iter()
            .find(|record| record.laio_name == laio_name))
    }

    pub(crate) fn read_by_session(&self, rex_session_id: &str) -> Result<Option<RexSessionRecord>> {
        let path = self.record_path(rex_session_id);

        match fs::read_to_string(&path) {
            Ok(json) => Ok(parse_record(&path, &json)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(miette!("failed to read '{}': {error}", path.display())),
        }
    }

    pub(crate) fn delete_by_session(&self, rex_session_id: &str) -> Result<()> {
        let path = self.record_path(rex_session_id);

        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(miette!("failed to delete '{}': {error}", path.display())),
        }
    }

    pub(crate) fn prune(&self, live_session_ids: &[String]) -> Result<Vec<String>> {
        let mut pruned = Vec::new();

        for record in self.records()? {
            if live_session_ids.contains(&record.rex_session_id) {
                continue;
            }

            self.delete_by_session(&record.rex_session_id)?;
            pruned.push(record.rex_session_id);
        }

        Ok(pruned)
    }

    pub(crate) fn list(&self) -> Result<Vec<RexSessionRecord>> {
        self.records()
    }

    fn records(&self) -> Result<Vec<RexSessionRecord>> {
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
            .filter_map(|path| read_record_file(path))
            .collect())
    }

    fn record_path(&self, rex_session_id: &str) -> PathBuf {
        let stem = rex_session_id
            .strip_prefix(SESSION_PREFIX)
            .unwrap_or(rex_session_id);

        self.dir.join(format!("{stem}.{RECORD_EXTENSION}"))
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
