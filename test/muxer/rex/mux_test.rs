use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};
use crate::common::config::Session;
use crate::muxer::rex::Rex;
use crate::muxer::rex::store::{RexSessionRecord, RexSessionStore};

pub(super) type TestResult<T = ()> = core::result::Result<T, Box<dyn std::error::Error>>;
pub(super) type Log = Arc<Mutex<Vec<String>>>;

pub(super) const WORK_KEY: &str = "00e13ed7";

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);
static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn unique_dir() -> PathBuf {
    let suffix = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());

    env::temp_dir().join(format!(
        "laio-rex-mux-{}-{nanos}-{suffix}",
        std::process::id()
    ))
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn env_guard() -> MutexGuard<'static, ()> {
    let guard = lock(&ENV_LOCK);

    unsafe {
        env::remove_var("REX_SESSION");
    }

    guard
}

pub(super) fn argv(cmd: &Type) -> (String, Vec<String>) {
    let command = match cmd {
        Type::Basic(command) | Type::Verbose(command) | Type::Forget(command) => command,
    };

    (
        command.get_program().to_string_lossy().into_owned(),
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
    )
}

pub(super) fn rex_args(cmd: &Type) -> Option<Vec<String>> {
    let (program, args) = argv(cmd);

    (program == "rex").then_some(args)
}

pub(super) fn api_payload(cmd: &Type, method: &str) -> Option<Value> {
    let args = rex_args(cmd)?;

    if args.len() != 4 || args[0] != "api" || args[1] != "call" || args[2] != method {
        return None;
    }

    serde_json::from_str(&args[3]).ok()
}

pub(super) fn expected(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

pub(super) fn session_of(yaml: &str) -> TestResult<Session> {
    Ok(noyalib::compat::serde_yaml::from_str(yaml)?)
}

pub(super) fn record(rex_session_id: &str, laio_name: &str) -> RexSessionRecord {
    RexSessionRecord {
        rex_session_id: rex_session_id.to_string(),
        laio_name: laio_name.to_string(),
        config_path: format!("/tmp/{laio_name}.yaml"),
        variables: format!("session_name={laio_name}&path=%2Ftmp"),
        created_at: 1_700_000_000,
    }
}

pub(super) struct Harness {
    cmd_unit: MockCmdUnitMock,
    cmd_string: MockCmdStringMock,
    store_dir: PathBuf,
}

impl Harness {
    pub(super) fn new() -> Self {
        Self {
            cmd_unit: MockCmdUnitMock::new(),
            cmd_string: MockCmdStringMock::new(),
            store_dir: unique_dir(),
        }
    }

    pub(super) fn build(self) -> Rex<RunnerMock> {
        Rex::new_with_parts(
            RunnerMock {
                cmd_unit: self.cmd_unit,
                cmd_string: self.cmd_string,
                cmd_bool: MockCmdBoolMock::new(),
            },
            self.store_dir,
        )
    }

    pub(super) fn store(&self) -> RexSessionStore {
        RexSessionStore::new(self.store_dir.clone())
    }

    pub(super) fn store_dir(&self) -> &Path {
        &self.store_dir
    }

    pub(super) fn seed(&self, record: &RexSessionRecord) -> TestResult {
        self.store().write_record(record)?;
        Ok(())
    }

    pub(super) fn create(mut self, label: &str, output: &str, windows: usize) -> Self {
        let label = label.to_string();
        let output = output.to_string();

        self.cmd_string
            .expect_run()
            .times(1)
            .withf(|cmd| api_payload(cmd, "session.create").is_some())
            .returning(move |cmd| {
                let payload = api_payload(cmd, "session.create").unwrap_or(Value::Null);

                assert_eq!(payload["label"].as_str(), Some(label.as_str()));
                assert_eq!(
                    payload["initial_windows"].as_array().map(Vec::len),
                    Some(windows)
                );

                Ok(output.clone())
            });

        self
    }

    pub(super) fn create_failing(mut self, detail: &str) -> Self {
        let detail = detail.to_string();

        self.cmd_string
            .expect_run()
            .times(1)
            .withf(|cmd| api_payload(cmd, "session.create").is_some())
            .returning(move |_| Err(miette::miette!(detail.clone())));

        self
    }

    pub(super) fn json_args(mut self, args: &[&str], output: &str) -> Self {
        let expected = expected(args);
        let output = output.to_string();

        self.cmd_string
            .expect_run()
            .times(1)
            .withf(move |cmd| rex_args(cmd).is_some_and(|actual| actual == expected))
            .returning(move |_| Ok(output.clone()));

        self
    }

    pub(super) fn json_recording(
        mut self,
        method: &'static str,
        payload: Value,
        output: &str,
        log: Log,
        label: &str,
    ) -> Self {
        let output = output.to_string();
        let label = label.to_string();

        self.cmd_string
            .expect_run()
            .times(1)
            .withf(move |cmd| api_payload(cmd, method).is_some_and(|actual| actual == payload))
            .returning(move |_| {
                lock(&log).push(label.clone());
                Ok(output.clone())
            });

        self
    }

    pub(super) fn hook_recording(mut self, line: &str, log: Log, label: &str) -> Self {
        let line = line.to_string();
        let label = label.to_string();

        self.cmd_string
            .expect_run()
            .times(1)
            .withf(move |cmd| matches!(cmd, Type::Verbose(_)) && cmd.to_string() == line)
            .returning(move |_| {
                lock(&log).push(label.clone());
                Ok(String::new())
            });

        self
    }

    pub(super) fn unit(mut self, args: &[&str]) -> Self {
        let expected = expected(args);

        self.cmd_unit
            .expect_run()
            .times(1)
            .withf(move |cmd| rex_args(cmd).is_some_and(|actual| actual == expected))
            .returning(|_| Ok(()));

        self
    }

    pub(super) fn unit_failing(mut self, args: &[&str], detail: &str) -> Self {
        let expected = expected(args);
        let detail = detail.to_string();

        self.cmd_unit
            .expect_run()
            .times(1)
            .withf(move |cmd| rex_args(cmd).is_some_and(|actual| actual == expected))
            .returning(move |_| Err(miette::miette!(detail.clone())));

        self
    }

    pub(super) fn unit_recording(mut self, args: &[&str], log: Log, label: &str) -> Self {
        let expected = expected(args);
        let label = label.to_string();

        self.cmd_unit
            .expect_run()
            .times(1)
            .withf(move |cmd| rex_args(cmd).is_some_and(|actual| actual == expected))
            .returning(move |_| {
                lock(&log).push(label.clone());
                Ok(())
            });

        self
    }

    pub(super) fn unit_asserting(
        mut self,
        args: &[&str],
        check: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let expected = expected(args);

        self.cmd_unit
            .expect_run()
            .times(1)
            .withf(move |cmd| rex_args(cmd).is_some_and(|actual| actual == expected))
            .returning(move |_| {
                check();
                Ok(())
            });

        self
    }

    pub(super) fn unit_at(mut self, args: &[&str], slot: Arc<Mutex<Option<Instant>>>) -> Self {
        let expected = expected(args);

        self.cmd_unit
            .expect_run()
            .times(1)
            .withf(move |cmd| rex_args(cmd).is_some_and(|actual| actual == expected))
            .returning(move |_| {
                *lock(&slot) = Some(Instant::now());
                Ok(())
            });

        self
    }
}
