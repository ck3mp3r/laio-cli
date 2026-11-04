use std::env;
use std::ffi::OsString;
use std::sync::MutexGuard;

use crate::common::config::FlexDirection;
use crate::common::muxer::Multiplexer;

use super::mux_test::*;

const VIEW: &str = r#"{
  "revision": 1,
  "session_id": "session:1",
  "label": "laio: work",
  "active_window_id": "w1",
  "windows": [
    {
      "window_id": "w1",
      "label": "main",
      "focused_block_id": "block:b2",
      "zoomed_block_id": "block:b1",
      "layers": [
        {
          "layer_id": "layer:tiled",
          "kind": "tiled",
          "bounds": [0.0, 0.0, 1.0, 1.0],
          "layout": {
            "rect": [0.0, 0.0, 1.0, 1.0],
            "split": {
              "direction": "horizontal",
              "ratio": 0.5,
              "before": {"rect": [0.0, 0.0, 0.5, 1.0], "block_id": "block:b1"},
              "after": {"rect": [0.5, 0.0, 0.5, 1.0], "block_id": "block:b2"}
            }
          },
          "blocks": [
            {"block_id": "block:b1", "label": "editor"},
            {"block_id": "block:b2"}
          ]
        }
      ]
    }
  ]
}"#;

struct RexSession {
    _guard: MutexGuard<'static, ()>,
    previous: Option<OsString>,
}

impl RexSession {
    fn new(value: &str) -> Self {
        let previous = env::var_os("REX_SESSION");
        let guard = env_guard();

        unsafe {
            env::set_var("REX_SESSION", value);
        }

        Self {
            _guard: guard,
            previous,
        }
    }
}

impl Drop for RexSession {
    fn drop(&mut self) {
        unsafe {
            match self.previous.take() {
                Some(value) => env::set_var("REX_SESSION", value),
                None => env::remove_var("REX_SESSION"),
            }
        }
    }
}

fn view_harness() -> Harness {
    Harness::new()
        .json_args(
            &[
                "api",
                "call",
                "session.view",
                r#"{"session_id":"session:1"}"#,
            ],
            VIEW,
        )
        .json_args(
            &["block", "call", "-s", "session:1", "-b", "block:b1", "pwd"],
            r#"{"pwd":"file://host/opt/project/a"}"#,
        )
        .json_args(
            &["block", "call", "-s", "session:1", "-b", "block:b2", "pwd"],
            r#"{"pwd":"file://host/opt/project/b"}"#,
        )
}

#[test]
fn get_session_bails_outside_a_rex_session() -> TestResult {
    let _guard = env_guard();
    let rex = Harness::new().build();

    let error = rex.get_session().err().ok_or("missing REX_SESSION")?;

    assert_eq!(
        error.to_string(),
        "You do not seem to be inside a rex session."
    );
    Ok(())
}

#[test]
fn get_session_exports_the_current_rex_session() -> TestResult {
    let _session = RexSession::new("session:1");
    let harness = view_harness();
    harness.seed(&record("session:1", "stored"))?;
    let rex = harness.build();

    let session = rex.get_session()?;

    assert_eq!(session.name, "stored");
    assert_eq!(session.path, "/opt/project");
    assert_eq!(session.windows.len(), 1);

    let window = &session.windows[0];
    assert_eq!(window.name, "main");
    assert_eq!(window.flex_direction, FlexDirection::Row);
    assert!(window.focus);
    assert_eq!(window.panes.len(), 2);

    assert_eq!(window.panes[0].flex, 1);
    assert_eq!(window.panes[0].name.as_deref(), Some("editor"));
    assert_eq!(window.panes[0].path, "a");
    assert!(window.panes[0].zoom);
    assert!(!window.panes[0].focus);

    assert_eq!(window.panes[1].flex, 1);
    assert_eq!(window.panes[1].name, None);
    assert_eq!(window.panes[1].path, "b");
    assert!(!window.panes[1].zoom);
    assert!(window.panes[1].focus);

    assert!(window.panes[1].commands.is_empty());
    Ok(())
}

#[test]
fn get_session_derives_the_name_from_the_label_without_a_record() -> TestResult {
    let _session = RexSession::new("session:1");
    let rex = view_harness().build();

    let session = rex.get_session()?;

    assert_eq!(session.name, "work");
    Ok(())
}
