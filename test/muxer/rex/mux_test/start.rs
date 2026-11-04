use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;

use crate::common::muxer::Multiplexer;

use super::mux_test::*;

const MULTI_WINDOW: &str = "
name: work
path: /tmp
windows:
  - name: main
    panes:
      - path: .
        commands:
          - command: echo
            args:
              - one
  - name: logs
";

const MULTI_CREATE: &str = r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1"],"revision":2},{"window_id":"window:2","block_ids":["block:2"],"revision":3}]}"#;

#[test]
fn start_creates_one_session_with_the_raw_name_and_records_it_before_sending() -> TestResult {
    let harness = Harness::new();
    let check_dir = harness.store_dir().to_path_buf();
    let check = Arc::new(move || {
        assert!(
            check_dir.join(format!("{WORK_KEY}.json")).is_file(),
            "the sidecar record must exist before the first send"
        );
    });

    let store = harness.store();
    let rex = harness
        .create("work", MULTI_CREATE, 2)
        .unit_asserting(
            &[
                "send",
                "-s",
                "session:1",
                "-b",
                "block:1",
                "--",
                "echo one\n",
            ],
            check,
        )
        .build();

    rex.start(
        &session_of(MULTI_WINDOW)?,
        &[("LAIO_CONFIG", "/tmp/work.yaml"), ("LAIO_VARS", "a=b")],
        true,
        false,
    )?;

    let record = store
        .read_by_name("work")?
        .ok_or("start should write the sidecar record")?;

    assert_eq!(record.rex_session_id, "session:1");
    assert_eq!(record.laio_name, "work");
    assert_eq!(record.config_path, "/tmp/work.yaml");
    assert_eq!(record.variables, "a=b");
    Ok(())
}

#[test]
fn start_returns_ok_without_creating_when_the_recorded_session_is_live() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(
            &["session", "ls", "--json"],
            r#"{"sessions":[{"session_id":"session:1","label":"work"}]}"#,
        )
        .build();

    Ok(rex.start(&session_of(MULTI_WINDOW)?, &[], true, false)?)
}

#[test]
fn start_creates_a_new_session_when_the_recorded_session_is_dead() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:9", "work"))?;
    let store = harness.store();

    let rex = harness
        .json_args(&["session", "ls", "--json"], r#"{"sessions":[]}"#)
        .create("work", MULTI_CREATE, 2)
        .build();

    rex.start(&session_of(MULTI_WINDOW)?, &[], true, true)?;

    assert_eq!(
        store
            .read_by_name("work")?
            .map(|record| record.rex_session_id),
        Some("session:1".to_string())
    );
    Ok(())
}

#[test]
fn start_sends_one_batch_per_leaf_with_a_trailing_newline() -> TestResult {
    let session = session_of(
        "
name: work
path: /tmp
windows:
  - name: main
    panes:
      - commands:
          - command: echo
            args:
              - one
      - commands:
          - command: echo
            args:
              - two
          - command: ls
",
    )?;

    let create = r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1","block:2"],"revision":2}]}"#;

    let rex = Harness::new()
        .create("work", create, 1)
        .unit(&[
            "send",
            "-s",
            "session:1",
            "-b",
            "block:1",
            "--",
            "echo one\n",
        ])
        .unit(&[
            "send",
            "-s",
            "session:1",
            "-b",
            "block:2",
            "--",
            "echo two\nls\n",
        ])
        .build();

    Ok(rex.start(&session, &[], true, false)?)
}

#[test]
fn start_waits_for_the_pane_command_delay_before_sending() -> TestResult {
    let session = session_of(
        "
name: work
path: /tmp
pane_cmd_delay: 40
windows:
  - name: main
    panes:
      - commands:
          - command: ls
",
    )?;

    let create = r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1"],"revision":2}]}"#;
    let sent_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    let rex = Harness::new()
        .create("work", create, 1)
        .unit_at(
            &["send", "-s", "session:1", "-b", "block:1", "--", "ls\n"],
            Arc::clone(&sent_at),
        )
        .build();

    let started = Instant::now();
    rex.start(&session, &[], true, false)?;

    let sent_at = lock(&sent_at).ok_or("the leaf commands should be sent")?;

    assert!(
        sent_at.duration_since(started) >= Duration::from_millis(40),
        "the send must wait for pane_cmd_delay"
    );
    Ok(())
}

#[test]
fn start_returns_err_without_a_record_when_session_create_fails() -> TestResult {
    let harness = Harness::new();
    let store = harness.store();

    let rex = harness
        .create_failing("rex session_create failed: bad_request")
        .build();

    let result = rex.start(&session_of(MULTI_WINDOW)?, &[], true, false);

    assert!(result.is_err());
    assert!(store.list()?.is_empty());
    Ok(())
}

#[test]
fn start_continues_when_one_send_fails() -> TestResult {
    let session = session_of(
        "
name: work
path: /tmp
windows:
  - name: main
    panes:
      - commands:
          - command: echo
            args:
              - one
      - commands:
          - command: ls
",
    )?;

    let create = r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1","block:2"],"revision":2}]}"#;

    let rex = Harness::new()
        .create("work", create, 1)
        .unit_failing(
            &[
                "send",
                "-s",
                "session:1",
                "-b",
                "block:1",
                "--",
                "echo one\n",
            ],
            "stream response failed: block target block:1 not found",
        )
        .unit(&["send", "-s", "session:1", "-b", "block:2", "--", "ls\n"])
        .build();

    Ok(rex.start(&session, &[], true, false)?)
}

#[test]
fn start_continues_when_session_select_fails() -> TestResult {
    let harness = Harness::new();
    harness.seed(&record("session:1", "work"))?;

    let rex = harness
        .json_args(
            &["session", "ls", "--json"],
            r#"{"sessions":[{"session_id":"session:1","label":"work"}]}"#,
        )
        .unit_failing(
            &["do", "session.select", "session_id=session:1"],
            "Rex.app is not accepting actions. Turn on Remote Control in its Settings under Rex Server.",
        )
        .build();

    Ok(rex.start(&session_of(MULTI_WINDOW)?, &[], false, false)?)
}

#[test]
fn start_skips_the_hooks_and_the_sends_when_skip_cmds_is_set() -> TestResult {
    let rex = Harness::new().create("work", MULTI_CREATE, 2).build();

    Ok(rex.start(&session_of(MULTI_WINDOW)?, &[], true, true)?)
}

#[test]
fn start_focuses_the_block_before_zooming_it() -> TestResult {
    let session = session_of(
        "
name: work
path: /tmp
windows:
  - name: main
    panes:
      - zoom: true
      - focus: true
",
    )?;

    let create = r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1","block:2"],"revision":2}]}"#;
    let log: Log = Arc::new(Mutex::new(Vec::new()));

    let rex = Harness::new()
        .create("work", create, 1)
        .json_recording(
            "session.focus_block",
            json!({"session_id": "session:1", "block_id": "block:2"}),
            r#"{"revision":4}"#,
            Arc::clone(&log),
            "focus_block",
        )
        .unit_recording(
            &["zoom", "-s", "session:1", "block:1", "--on"],
            Arc::clone(&log),
            "zoom",
        )
        .build();

    rex.start(&session, &[], true, true)?;

    assert_eq!(lock(&log).as_slice(), ["focus_block", "zoom"]);
    Ok(())
}

#[test]
fn start_focuses_the_window_that_asks_for_it() -> TestResult {
    let session = session_of(
        "
name: work
path: /tmp
windows:
  - name: main
  - name: logs
    focus: true
",
    )?;

    let rex = Harness::new()
        .create("work", MULTI_CREATE, 2)
        .json_recording(
            "session.focus_window",
            json!({"session_id": "session:1", "window_id": "window:2"}),
            r#"{"revision":4}"#,
            Arc::new(Mutex::new(Vec::new())),
            "focus_window",
        )
        .build();

    Ok(rex.start(&session, &[], true, true)?)
}

#[test]
fn start_accepts_a_unicode_name_and_records_it_verbatim() -> TestResult {
    let harness = Harness::new();
    let store = harness.store();
    let rex = harness.create("Über-Foo", MULTI_CREATE, 2).build();

    rex.start(
        &session_of("\nname: Über-Foo\npath: /tmp\nwindows:\n  - name: main\n  - name: logs\n")?,
        &[],
        true,
        true,
    )?;

    let record = store
        .read_by_name("Über-Foo")?
        .ok_or("start should record the unicode name")?;

    assert_eq!(record.laio_name, "Über-Foo");
    Ok(())
}

#[test]
fn start_rejects_an_empty_name() -> TestResult {
    let rex = Harness::new().build();

    let error = rex
        .start(
            &session_of("\nname: ''\npath: /tmp\nwindows:\n  - name: main\n")?,
            &[],
            true,
            true,
        )
        .err()
        .ok_or("an empty name should fail")?;

    assert!(
        error.to_string().contains("Invalid rex session name"),
        "{error}"
    );
    Ok(())
}

#[test]
fn start_accepts_a_name_at_the_128_byte_limit_and_rejects_one_over() -> TestResult {
    let allowed = "Ü".repeat(64);
    let rex = Harness::new()
        .create(
            &allowed,
            r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1"],"revision":2}]}"#,
            1,
        )
        .build();

    rex.start(
        &session_of(&format!(
            "\nname: {allowed}\npath: /tmp\nwindows:\n  - name: main\n"
        ))?,
        &[],
        true,
        true,
    )?;

    let rejected = "b".repeat(129);
    let rex = Harness::new().build();
    let error = rex
        .start(
            &session_of(&format!(
                "\nname: {rejected}\npath: /tmp\nwindows:\n  - name: main\n"
            ))?,
            &[],
            true,
            true,
        )
        .err()
        .ok_or("a name one over the label limit should fail")?;

    assert!(error.to_string().contains("128"), "{error}");
    Ok(())
}

#[test]
fn start_continues_when_the_record_cannot_be_written() -> TestResult {
    let harness = Harness::new();
    let blocked = harness.store_dir().to_path_buf();
    fs::write(&blocked, "not a directory")?;

    let rex = harness.create("work", MULTI_CREATE, 2).build();

    Ok(rex.start(&session_of(MULTI_WINDOW)?, &[], true, true)?)
}

#[test]
fn start_bails_before_creating_anything_when_the_record_key_belongs_to_another_name() -> TestResult
{
    let harness = Harness::new();
    harness.seed(&record("session:1", "probe-1803"))?;
    let store = harness.store();
    let rex = harness.build();

    let error = rex
        .start(
            &session_of("\nname: probe-89065\npath: /tmp\nwindows:\n  - name: main\n")?,
            &[],
            true,
            true,
        )
        .err()
        .ok_or("a record key collision should fail the start")?;
    let message = error.to_string();

    assert!(message.contains("probe-1803"), "{message}");
    assert!(message.contains("probe-89065"), "{message}");

    let records = store.list()?;

    assert_eq!(records.len(), 1, "no sidecar record is written");
    assert_eq!(
        records.first().map(|record| record.rex_session_id.as_str()),
        Some("session:1"),
        "the stored record keeps its owner"
    );
    Ok(())
}
