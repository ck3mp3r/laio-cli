use std::env;
use std::sync::MutexGuard;

use crate::common::muxer::Multiplexer;
use crate::common::session_info::{SessionInfo, SessionStatus};

use super::mux_test::*;

const SESSION_LIST: &str = r#"{"sessions":[{"session_id":"session:1","label":"work"},{"session_id":"session:2","label":"extra"},{"session_id":"session:3","label":"other"}]}"#;

struct RexSessionEnv {
    _guard: MutexGuard<'static, ()>,
}

impl RexSessionEnv {
    fn set(value: &str) -> Self {
        let guard = env_guard();

        unsafe {
            env::set_var("REX_SESSION", value);
        }

        Self { _guard: guard }
    }

    fn unset() -> Self {
        Self {
            _guard: env_guard(),
        }
    }
}

impl Drop for RexSessionEnv {
    fn drop(&mut self) {
        unsafe {
            env::remove_var("REX_SESSION");
        }
    }
}

fn rows(sessions: &[SessionInfo]) -> Vec<(&str, SessionStatus)> {
    let mut rows: Vec<(&str, SessionStatus)> = sessions
        .iter()
        .map(|info| (info.name.as_str(), info.status))
        .collect();

    rows.sort_by_key(|(name, _)| *name);
    rows
}

#[test]
fn get_session_config_path_returns_the_recorded_path() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    let rex = harness.build();

    assert_eq!(
        rex.get_session_config_path("work")?,
        Some("/tmp/work.yaml".to_string())
    );
    assert_eq!(rex.get_session_config_path("missing")?, None);
    Ok(())
}

#[test]
fn get_session_variables_returns_the_decoded_variables() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    let rex = harness.build();

    assert_eq!(
        rex.get_session_variables("work")?,
        Some(vec![
            "session_name=work".to_string(),
            "path=/tmp".to_string()
        ])
    );
    assert_eq!(rex.get_session_variables("missing")?, None);
    Ok(())
}

#[test]
fn get_session_variables_returns_none_for_undecodable_variables() -> TestResult {
    let harness = Harness::new();
    let mut broken = record("session:1", "work");
    broken.variables = "not-a-pair".to_string();
    harness.seed(&broken)?;

    assert_eq!(harness.build().get_session_variables("work")?, None);
    Ok(())
}

#[test]
fn list_sessions_reports_only_live_recorded_sessions() -> TestResult {
    let _env = RexSessionEnv::unset();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    harness.seed(&record("session:9", "stale"))?;
    let store = harness.store();

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .build();

    let sessions = rex.list_sessions()?;

    assert_eq!(rows(&sessions), [("work", SessionStatus::Active)]);
    assert!(
        store.read_by_name("stale")?.is_none(),
        "the stale record file must be deleted"
    );
    Ok(())
}

#[test]
fn list_sessions_marks_the_session_laio_runs_in_as_attached() -> TestResult {
    let _env = RexSessionEnv::set("session:1");
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    harness.seed(&record("session:2", "extra"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .build();

    let sessions = rex.list_sessions()?;

    assert_eq!(
        rows(&sessions),
        [
            ("extra", SessionStatus::Active),
            ("work", SessionStatus::Attached)
        ]
    );
    Ok(())
}

#[test]
fn list_sessions_reports_every_live_session_active_outside_a_rex_session() -> TestResult {
    let _env = RexSessionEnv::unset();
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    harness.seed(&record("session:2", "extra"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .build();

    let sessions = rex.list_sessions()?;

    assert_eq!(
        rows(&sessions),
        [
            ("extra", SessionStatus::Active),
            ("work", SessionStatus::Active)
        ]
    );
    Ok(())
}

#[test]
fn list_sessions_marks_no_session_attached_for_a_foreign_rex_session() -> TestResult {
    let _env = RexSessionEnv::set("session:404");
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    harness.seed(&record("session:2", "extra"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .build();

    let sessions = rex.list_sessions()?;

    assert_eq!(
        rows(&sessions),
        [
            ("extra", SessionStatus::Active),
            ("work", SessionStatus::Active)
        ]
    );
    Ok(())
}
