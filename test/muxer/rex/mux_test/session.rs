use crate::common::muxer::Multiplexer;
use crate::common::session_info::SessionStatus;

use super::mux_test::*;

const SESSION_LIST: &str = r#"{"sessions":[{"session_id":"session:1","label":"laio: work"},{"session_id":"session:2","label":"laio: extra"},{"session_id":"session:3","label":"other"}]}"#;

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
fn list_sessions_prunes_dead_records_and_marks_unrecorded_labels_inactive() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;
    harness.seed(&record("session:9", "stale"))?;
    let store = harness.store();

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .json_args(&["client", "ls", "--json"], r#"{"clients":[]}"#)
        .build();

    let sessions = rex.list_sessions()?;
    let rows: Vec<(&str, SessionStatus)> = sessions
        .iter()
        .map(|info| (info.name.as_str(), info.status))
        .collect();

    assert_eq!(
        rows,
        [
            ("work", SessionStatus::Active),
            ("extra", SessionStatus::Inactive)
        ]
    );
    assert!(
        store.read_by_session("session:9")?.is_none(),
        "the stale record file must be deleted"
    );
    Ok(())
}

#[test]
fn list_sessions_reports_a_client_held_session_as_attached() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let clients = r#"{"clients":[{"client_id":"client:app","info":{"kind":"app"},"session_ids":["session:1"]}]}"#;

    let rex = harness
        .json_args(&["session", "ls", "--json"], SESSION_LIST)
        .json_args(&["client", "ls", "--json"], clients)
        .build();

    let sessions = rex.list_sessions()?;

    assert_eq!(
        sessions.first().map(|info| info.status),
        Some(SessionStatus::Attached)
    );
    Ok(())
}
