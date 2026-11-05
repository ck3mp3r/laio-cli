use crate::common::muxer::Multiplexer;

use super::mux_test::*;

const LIVE_WORK: &str = r#"{"sessions":[{"session_id":"session:1","label":"work"}]}"#;

#[test]
fn switch_returns_false_without_a_record_and_never_asks_the_server() -> TestResult {
    let rex = Harness::new().build();

    assert!(!rex.switch("work", true)?);
    Ok(())
}

#[test]
fn switch_returns_true_and_skips_attach_for_a_live_recorded_session() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], LIVE_WORK)
        .build();

    assert!(rex.switch("work", true)?);
    Ok(())
}

#[test]
fn switch_attaches_with_the_recorded_session_id() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], LIVE_WORK)
        .unit(&["do", "session.select", "session_id=session:1"])
        .build();

    assert!(rex.switch("work", false)?);
    Ok(())
}

#[test]
fn switch_continues_when_the_attach_fails() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], LIVE_WORK)
        .unit_failing(
            &["do", "session.select", "session_id=session:1"],
            "Rex.app is not accepting actions. Turn on Remote Control in its Settings under Rex Server.",
        )
        .build();

    assert!(rex.switch("work", false)?);
    Ok(())
}

#[test]
fn switch_returns_false_when_the_recorded_session_is_not_live() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(&["session", "ls", "--json"], r#"{"sessions":[]}"#)
        .build();

    assert!(!rex.switch("work", true)?);
    Ok(())
}
