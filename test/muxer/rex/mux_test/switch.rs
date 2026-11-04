use crate::common::muxer::Multiplexer;
use crate::muxer::rex::Rex;

use super::mux_test::*;

#[test]
fn switch_returns_true_and_skips_attach_when_requested() -> TestResult {
    let rex = Harness::new()
        .resolve("laio-work", "resolved", "session:1")
        .build();

    assert!(rex.switch("work", true)?);
    Ok(())
}

#[test]
fn switch_attaches_with_the_full_prefixed_session_id() -> TestResult {
    let rex = Harness::new()
        .resolve("laio-work", "resolved", "session:1")
        .unit(&["do", "session.select", "session_id=session:1"])
        .build();

    assert!(rex.switch("work", false)?);
    Ok(())
}

#[test]
fn switch_continues_when_the_attach_fails() -> TestResult {
    let rex = Harness::new()
        .resolve("laio-work", "resolved", "session:1")
        .unit_failing(
            &["do", "session.select", "session_id=session:1"],
            "Rex.app is not accepting actions. Turn on Remote Control in its Settings under Rex Server.",
        )
        .build();

    assert!(rex.switch("work", false)?);
    Ok(())
}

#[test]
fn switch_rejects_an_ambiguous_target_with_the_label() -> TestResult {
    let rex = Harness::new().resolve("laio-work", "ambiguous", "").build();

    let error = rex
        .switch("work", true)
        .err()
        .ok_or("ambiguity should fail")?;

    assert!(error.to_string().contains("laio-work"), "{error}");
    Ok(())
}

#[test]
fn switch_returns_false_when_the_target_is_not_found() -> TestResult {
    let rex = Harness::new().resolve("laio-work", "not_found", "").build();

    assert!(!rex.switch("work", true)?);
    Ok(())
}

#[test]
fn switch_rejects_names_outside_the_laio_charset() -> TestResult {
    let rex = Rex::new();

    let error = rex.switch("bad/name", true).err().ok_or("invalid name")?;

    assert!(error.to_string().contains("Invalid"), "{error}");
    Ok(())
}

#[test]
fn switch_rejects_a_label_over_the_rex_limit() -> TestResult {
    let rex = Rex::new();
    let name = "a".repeat(124);

    let error = rex
        .switch(&name, true)
        .err()
        .ok_or("an over-long label should fail")?;

    assert!(error.to_string().contains("128"), "{error}");
    Ok(())
}
