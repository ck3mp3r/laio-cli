use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use miette::{Result, bail};

use crate::app::manager::session::manager::{LAIO_CONFIG, LAIO_VARS, decode_variables};
use crate::cmd_basic;
use crate::common::cmd::{Runner, ShellRunner, Type};
use crate::common::config::Session;
use crate::common::muxer::{Client, Multiplexer};
use crate::common::path::{resolve_symlink, to_absolute_path};
use crate::common::session_info::SessionInfo;
use crate::muxer::rex::{
    LeafPlan, build_create_request, local_path, session_from_view, tiled_block_ids,
};

use super::client::RexClient;
use super::model::CreateOutput;
use super::store::{RexSessionRecord, RexSessionStore, laio_state_dir};

const MAX_LABEL_LEN: usize = 128;
const REX_SESSION_ENV: &str = "REX_SESSION";

pub(crate) struct Rex<R: Runner = ShellRunner> {
    client: RexClient<R>,
    store: RexSessionStore,
}

impl Rex {
    pub(crate) fn new() -> Self {
        Self::new_with_parts(ShellRunner::new(), laio_state_dir())
    }
}

impl<R: Runner> Rex<R> {
    pub(crate) fn new_with_parts(runner: R, store_dir: PathBuf) -> Self {
        Self {
            client: RexClient::new(Arc::new(runner)),
            store: RexSessionStore::new(store_dir),
        }
    }

    fn ensure_record_key_is_free(&self, name: &str) -> Result<()> {
        match self.store.stored_name_at_key(name) {
            Ok(Some(stored)) if stored != name => {
                bail!(
                    "Refusing to start '{name}': its rex session record key already belongs to '{stored}'. Rename one of the two sessions."
                );
            }
            Ok(_) => Ok(()),
            Err(error) => {
                log::warn!("Failed to check the rex session record for '{name}': {error}");
                Ok(())
            }
        }
    }

    fn write_record(&self, rex_session_id: &str, session: &Session, env_vars: &[(&str, &str)]) {
        let record = RexSessionRecord {
            rex_session_id: rex_session_id.to_string(),
            laio_name: session.name.clone(),
            config_path: env_value(env_vars, LAIO_CONFIG).to_string(),
            variables: env_value(env_vars, LAIO_VARS).to_string(),
            created_at: unix_seconds(),
        };

        if let Err(error) = self.store.write_record(&record) {
            log::warn!(
                "Failed to write the rex session record for '{}': {error}",
                session.name
            );
        }
    }

    fn delete_record(&self, name: &str) {
        if let Err(error) = self.store.delete_by_name(name) {
            log::warn!("Failed to delete the rex session record '{name}': {error}");
        }
    }

    fn run_startup_hooks(&self, session: &Session) -> Result<()> {
        let mut commands = session.startup.clone();
        if let Some(script) = &session.startup_script {
            commands.push(script.to_cmd()?);
        }

        self.client.run_commands(&commands, &session.path)
    }

    fn run_shutdown_hooks(
        &self,
        name: &str,
        record: &RexSessionRecord,
        session: &Option<Session>,
    ) -> Result<()> {
        if let Some(session) = session {
            return self.run_shutdown_commands(session);
        }

        let variables = decode_record_variables(name, &record.variables);
        let session = Session::from_config(
            &resolve_symlink(&to_absolute_path(&record.config_path)?)?,
            variables.as_deref(),
        )?;

        self.run_shutdown_commands(&session)
    }

    fn run_shutdown_commands(&self, session: &Session) -> Result<()> {
        let mut commands = session.shutdown.clone();
        if let Some(script) = &session.shutdown_script {
            commands.push(script.to_cmd()?);
        }

        self.client.run_commands(&commands, &session.path)
    }

    fn attach(&self, rex_session_id: &str, skip_attach: bool) {
        if skip_attach {
            return;
        }

        if let Err(error) = self.select_session(rex_session_id) {
            log::warn!("Failed to attach to rex session '{rex_session_id}': {error}");
        }
    }

    fn select_session(&self, rex_session_id: &str) -> Result<()> {
        let _: () = self.client.get_runner().run(&cmd_basic!(
            "rex",
            args = [
                "do",
                "session.select",
                format!("session_id={rex_session_id}")
            ]
        ))?;

        Ok(())
    }

    fn kill_session(&self, rex_session_id: &str) -> Result<()> {
        let _: () = self
            .client
            .get_runner()
            .run(&cmd_basic!("rex", args = ["kill", rex_session_id]))?;

        Ok(())
    }

    fn stop_named(
        &self,
        name: &str,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_other: bool,
    ) -> Result<()> {
        let Some(record) = self.store.read_by_name(name)? else {
            log::debug!("Not a laio session: {name}");
            return Ok(());
        };

        self.stop_resolved(
            &record.laio_name,
            &record.rex_session_id,
            &record,
            session,
            skip_cmds,
            stop_other,
        )
    }

    fn stop_current(
        &self,
        rex_session_id: &str,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_other: bool,
    ) -> Result<()> {
        let Some(record) = self.store.find_by_session_id(rex_session_id)? else {
            log::debug!("No laio record for the current rex session: {rex_session_id}");
            return Ok(());
        };

        self.stop_resolved(
            &record.laio_name,
            rex_session_id,
            &record,
            session,
            skip_cmds,
            stop_other,
        )
    }

    fn name_from_session_id(&self, rex_session_id: &str) -> Result<Option<String>> {
        Ok(self
            .store
            .find_by_session_id(rex_session_id)?
            .map(|record| record.laio_name))
    }

    fn stop_resolved(
        &self,
        name: &str,
        rex_session_id: &str,
        record: &RexSessionRecord,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_other: bool,
    ) -> Result<()> {
        log::trace!("Closing rex session: {name}");

        let hooks = if skip_cmds || stop_other {
            Ok(())
        } else {
            self.run_shutdown_hooks(name, record, session)
        };

        let kill = self.kill_session(rex_session_id);
        self.delete_record(name);

        hooks.and(kill)
    }

    fn sweep(&self, skip_cmds: bool, current: Option<&str>) -> Result<()> {
        log::trace!("Closing all/other laio rex sessions.");

        let live_ids: Vec<String> = self
            .client
            .session_list()?
            .sessions
            .into_iter()
            .map(|session| session.session_id)
            .collect();

        self.store.prune(&live_ids)?;

        let targets: Vec<(String, String, RexSessionRecord)> = self
            .store
            .list()?
            .into_iter()
            .filter(|record| Some(record.rex_session_id.as_str()) != current)
            .map(|record| {
                (
                    record.laio_name.clone(),
                    record.rex_session_id.clone(),
                    record,
                )
            })
            .collect();

        for (name, rex_session_id, record) in &targets {
            log::trace!("Closing rex session: {name}");

            let hooks = if skip_cmds {
                Ok(())
            } else {
                self.run_shutdown_hooks(name, record, &None)
            };

            let kill = self.kill_session(rex_session_id);
            self.delete_record(name);

            hooks.and(kill)?;
        }

        Ok(())
    }
}

impl<R: Runner> Multiplexer for Rex<R> {
    fn start(
        &self,
        session: &Session,
        env_vars: &[(&str, &str)],
        skip_attach: bool,
        skip_cmds: bool,
    ) -> Result<()> {
        self.ensure_record_key_is_free(&session.name)?;

        if self.switch(&session.name, skip_attach)? {
            return Ok(());
        }

        let label = laio_label(&session.name)?;
        let (mut request, leaves) = build_create_request(session)?;
        request.label = Some(label);

        let created = self.client.session_create(&request)?;

        self.write_record(&created.session_id, session, env_vars);

        if !skip_cmds {
            self.run_startup_hooks(session)?;
        }

        if let Some(index) = session.windows.iter().position(|window| window.focus)
            && let Some(window) = created.initial_windows.get(index)
        {
            self.client
                .session_focus_window(&created.session_id, &window.window_id)?;
        }

        for leaf in leaves.iter().filter(|leaf| leaf.focus) {
            if let Some(block_id) = block_id(&created, leaf) {
                self.client
                    .session_focus_block(&created.session_id, block_id)?;
            }
        }

        for leaf in leaves.iter().filter(|leaf| leaf.zoom) {
            if let Some(block_id) = block_id(&created, leaf) {
                self.client.zoom(&created.session_id, block_id, true)?;
            }
        }

        if !skip_cmds {
            if let Some(delay_ms) = session.pane_cmd_delay {
                thread::sleep(Duration::from_millis(delay_ms));
            }

            for leaf in &leaves {
                if leaf.commands.is_empty() {
                    continue;
                }

                let Some(block_id) = block_id(&created, leaf) else {
                    log::warn!(
                        "rex session '{}' has no block for window {} index {}",
                        session.name,
                        leaf.window_index,
                        leaf.block_index
                    );
                    continue;
                };

                let text = format!("{}\n", leaf.commands.join("\n"));

                if let Err(error) = self
                    .client
                    .send_commands(&created.session_id, block_id, &text)
                {
                    log::warn!("Failed to send commands to rex block '{block_id}': {error}");
                }
            }
        }

        self.attach(&created.session_id, skip_attach);

        Ok(())
    }

    fn stop(
        &self,
        name: &Option<String>,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()> {
        let current = current_session();
        log::trace!("Current rex session: {current:?}");

        if !stop_all && !stop_other && name.is_none() && current.is_none() {
            bail!("Specify laio session you want to stop.");
        }

        if (stop_all || stop_other) && name.is_some() {
            bail!("Stopping all/other and specifying a session name are mutually exclusive.");
        }

        if stop_other && current.is_none() {
            bail!("Not inside a rex session.");
        }

        if stop_all || (stop_other && current.is_some()) {
            return self.sweep(skip_cmds, current.as_deref());
        }

        if let Some(name) = name {
            return self.stop_named(name, session, skip_cmds, stop_other);
        }

        let Some(current) = current else {
            return Ok(());
        };

        self.stop_current(&current, session, skip_cmds, stop_other)
    }

    fn get_session_config_path(&self, name: &str) -> Result<Option<String>> {
        Ok(self
            .store
            .read_by_name(name)?
            .map(|record| record.config_path))
    }

    fn get_session_variables(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some(record) = self.store.read_by_name(name)? else {
            return Ok(None);
        };

        Ok(decode_record_variables(name, &record.variables))
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let live_ids: HashSet<String> = self
            .client
            .session_list()?
            .sessions
            .into_iter()
            .map(|session| session.session_id)
            .collect();

        let live: Vec<String> = live_ids.iter().cloned().collect();
        self.store.prune(&live)?;

        let current = current_session();

        Ok(self
            .store
            .list()?
            .into_iter()
            .filter(|record| live_ids.contains(&record.rex_session_id))
            .map(|record| {
                let attached = current.as_deref() == Some(record.rex_session_id.as_str());
                SessionInfo::active(record.laio_name, attached)
            })
            .collect())
    }

    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        let record = match self.store.read_by_name(name) {
            Ok(record) => record,
            Err(error) => {
                log::warn!("Failed to read the rex session record for '{name}': {error}");
                return Ok(false);
            }
        };

        let Some(record) = record else {
            return Ok(false);
        };

        let live = self
            .client
            .session_list()?
            .sessions
            .iter()
            .any(|session| session.session_id == record.rex_session_id);

        if !live {
            return Ok(false);
        }

        self.attach(&record.rex_session_id, skip_attach);

        Ok(true)
    }

    fn get_session(&self) -> Result<Session> {
        let Some(rex_session_id) = current_session() else {
            bail!("You do not seem to be inside a rex session.");
        };

        let view = self.client.session_view(&rex_session_id)?;
        let mut pwds = HashMap::new();

        for block_id in tiled_block_ids(&view)? {
            match self.client.block_pwd(&rex_session_id, block_id) {
                Ok(output) => {
                    pwds.insert(block_id.to_string(), local_path(&output.pwd).to_string());
                }
                Err(error) => {
                    log::warn!(
                        "Failed to read the working directory of rex block '{block_id}': {error}"
                    );
                }
            }
        }

        let name = self.name_from_session_id(&rex_session_id)?;

        session_from_view(&view, &pwds, name.as_deref())
    }
}

fn block_id<'a>(created: &'a CreateOutput, leaf: &LeafPlan) -> Option<&'a str> {
    created
        .initial_windows
        .get(leaf.window_index)
        .and_then(|window| window.block_ids.get(leaf.block_index))
        .map(String::as_str)
}

fn decode_record_variables(name: &str, encoded: &str) -> Option<Vec<String>> {
    if encoded.is_empty() {
        return Some(Vec::new());
    }

    match decode_variables(encoded) {
        Ok(variables) => Some(variables),
        Err(error) => {
            log::warn!("Failed to decode LAIO_VARS for session '{name}': {error:?}");
            None
        }
    }
}

fn laio_label(name: &str) -> Result<String> {
    if name.is_empty() {
        bail!("Invalid rex session name '{name}'");
    }

    if name.len() > MAX_LABEL_LEN {
        bail!("rex session label '{name}' exceeds {MAX_LABEL_LEN} bytes");
    }

    Ok(name.to_string())
}

fn env_value<'a>(env_vars: &'a [(&str, &str)], key: &str) -> &'a str {
    env_vars
        .iter()
        .find(|(env_key, _)| *env_key == key)
        .map(|(_, value)| *value)
        .unwrap_or_default()
}

fn current_session() -> Option<String> {
    env::var(REX_SESSION_ENV)
        .ok()
        .filter(|value| !value.is_empty())
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
