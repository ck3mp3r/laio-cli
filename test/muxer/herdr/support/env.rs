use std::{
    env,
    ffi::OsString,
    fs,
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use miette::{IntoDiagnostic, Result, miette};

use crate::common::cmd::test::RunnerMock;

use super::client::{HerdrClient, resolve_socket_path};

use super::support_socket::*;

pub(super) static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(super) const GUARDED_ENV: [&str; 3] = ["HERDR_ENV", "HERDR_SOCKET_PATH", "HERDR_WORKSPACE_ID"];

pub(super) struct EnvGuard {
    _guard: MutexGuard<'static, ()>,
    saved: [(&'static str, Option<OsString>); 3],
}

impl EnvGuard {
    pub(super) fn new() -> Self {
        let guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = GUARDED_ENV.map(|key| {
            let previous = env::var_os(key);

            unsafe {
                env::remove_var(key);
            }

            (key, previous)
        });

        Self {
            _guard: guard,
            saved,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.saved.iter_mut() {
            restore_env(key, previous.take());
        }
    }
}

pub(super) fn env_lock() -> Result<EnvGuard> {
    Ok(EnvGuard::new())
}

pub(super) fn restore_env(key: &str, previous: Option<OsString>) {
    unsafe {
        match previous {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
}

pub(super) static SESSION_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(super) struct TestSession {
    _guard: EnvGuard,
    xdg_config_home: PathBuf,
    previous_xdg: Option<OsString>,
    name: String,
}

impl TestSession {
    pub(super) fn new() -> Result<Self> {
        let guard = env_lock()?;
        let id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let xdg_config_home = env::temp_dir().join(format!("lh{}-{id}", std::process::id()));
        let previous_xdg = env::var_os("XDG_CONFIG_HOME");

        unsafe {
            env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
        }

        Ok(Self {
            _guard: guard,
            xdg_config_home,
            previous_xdg,
            name: "w".to_string(),
        })
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn socket_path(&self) -> Result<PathBuf> {
        resolve_socket_path(&self.name)
    }

    pub(super) fn bind(&self) -> Result<UnixListener> {
        let path = self.socket_path()?;
        let parent = path
            .parent()
            .ok_or_else(|| miette!("socket path should have a parent directory"))?;
        fs::create_dir_all(parent).into_diagnostic()?;
        bind_listener(&path)
    }

    pub(super) fn client(&self) -> HerdrClient<RunnerMock> {
        HerdrClient::new_with_runner(empty_runner())
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        restore_env("XDG_CONFIG_HOME", self.previous_xdg.take());
        let _ = fs::remove_dir_all(&self.xdg_config_home);
    }
}

pub(super) fn clear_herdr_socket_path() -> Option<OsString> {
    let previous = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::remove_var("HERDR_SOCKET_PATH");
    }

    previous
}

pub(super) fn set_herdr_socket_path(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::set_var("HERDR_SOCKET_PATH", value);
    }

    previous
}

pub(super) fn clear_herdr_workspace_id() -> Option<OsString> {
    let previous = env::var_os("HERDR_WORKSPACE_ID");

    unsafe {
        env::remove_var("HERDR_WORKSPACE_ID");
    }

    previous
}

pub(super) fn set_herdr_workspace_id(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_WORKSPACE_ID");

    unsafe {
        env::set_var("HERDR_WORKSPACE_ID", value);
    }

    previous
}

pub(super) fn clear_herdr_env() -> Option<OsString> {
    let previous = env::var_os("HERDR_ENV");

    unsafe {
        env::remove_var("HERDR_ENV");
    }

    previous
}

pub(super) fn set_herdr_env(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_ENV");

    unsafe {
        env::set_var("HERDR_ENV", value);
    }

    previous
}
