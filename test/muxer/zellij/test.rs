use crate::common::{
    cmd::{
        Type,
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
    },
    config::{Command, FlexDirection, Pane, Script, Session, Window},
    muxer::Multiplexer,
};
use miette::{IntoDiagnostic, Result};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use super::Zellij;
use super::client::ZellijClient;
use super::mux::resolve_shell;

#[test]
fn mux_start_session() -> Result<()> {
    let temp_dir = std::env::temp_dir();
    let temp_dir_lossy = temp_dir.to_string_lossy();
    let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
    let yaml_str = include_str!("../../../src/common/config/fixtures/valid.yaml")
        .replace("/tmp", temp_dir_str);
    let session: Session = noyalib::compat::serde_yaml::from_str(&yaml_str).unwrap();

    let path_str = "./common/config/fixtures/valid.yaml";

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    let expected_create_prefix = format!(
        "LAIO_CONFIG={path_str} LAIO_VARS= sh -c nohup zellij --session valid --new-session-with-layout"
    );

    cmd_unit
        .expect_run()
        .times(1)
        .withf(move |cmd| {
            matches!(cmd, Type::Forget(_) if
                cmd.to_string().starts_with(&expected_create_prefix)
                && cmd.to_string().contains("2>"))
        })
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Forget(_) if cmd.to_string() == "zellij attach valid"))
        .returning(|_| Ok(()));

    cmd_bool
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c zellij list-sessions --short | grep \"valid\""))
        .returning({
            let mut calls = 0;
            move |_| {
                calls += 1;
                Ok(calls > 1)
            }
        });

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Verbose(_) if ["date", "echo Hi"].contains(&cmd.to_string().as_str())),
        )
        .returning(|_| Ok("".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string().contains("laio-277d3966f692fca8534baf09ce5fc483c928868d776993609681f6d524184281")))
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    let env_vars: Vec<(&str, &str)> = vec![("LAIO_CONFIG", path_str), ("LAIO_VARS", "")];

    zellij.start(&session, &env_vars, false, false)?;

    let mut layout_path = std::env::temp_dir();
    layout_path.push("valid.kdl");
    assert!(
        !layout_path.exists(),
        "the layout temp file should be deleted: {}",
        layout_path.display()
    );

    let mut error_path = std::env::temp_dir();
    error_path.push("valid.zellij.err");
    assert!(
        !error_path.exists(),
        "the stderr temp file should be deleted: {}",
        error_path.display()
    );

    Ok(())
}
#[test]
#[ignore = "flaky on GitHub Actions due to runner environment issues"]
fn mux_stop_session() -> Result<()> {
    let path_str = format!(
        "{}/src/common/config/fixtures/valid.yaml",
        env!("CARGO_MANIFEST_DIR")
    );

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c printenv LAIO_CONFIG || true"),
        )
        .returning({
            move |_| Ok(path_str.to_string())
        });

    cmd_string
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "printenv ZELLIJ"))
        .returning(|_| Ok("0".to_string()));

    cmd_bool
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c zellij list-sessions --short | grep \"valid\""))
        .returning(|_| Ok(true));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "zellij delete-session valid --force"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if ["date", "echo Bye"].contains(&cmd.to_string().as_str())))
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);
    println!("DEBUG: About to call stop()");
    zellij.stop(&Some("valid".to_string()), &None, false, false, false)?;
    println!("DEBUG: stop() completed");

    Ok(())
}

#[test]
fn mux_get_session() -> Result<()> {
    let to_yaml = |yaml: String| -> Result<String> {
        let tmp_yaml: noyalib::compat::serde_yaml::Value =
            noyalib::compat::serde_yaml::from_str(yaml.as_str()).into_diagnostic()?;
        let string_yaml = noyalib::compat::serde_yaml::to_string(&tmp_yaml).into_diagnostic()?;
        Ok(string_yaml)
    };

    let valid_yaml =
        to_yaml(include_str!("../../../src/common/config/fixtures/to_yaml.yaml").to_string())?;
    let valid_kdl = include_str!("../../../src/common/config/fixtures/to_yaml.kdl").to_string();

    let cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "printenv ZELLIJ"))
        .returning(|_| Ok("0".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "zellij action dump-layout"),
        )
        .returning(move |_| Ok(valid_kdl.to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);
    let result = zellij.get_session()?;

    let expected_session_yaml =
        to_yaml(noyalib::compat::serde_yaml::to_string(&result).into_diagnostic()?)?;
    assert_eq!(valid_yaml, expected_session_yaml);
    Ok(())
}

fn render_session(commands: Vec<Command>, script: Option<Script>, shell: &str) -> Result<String> {
    let pane = Pane {
        flex_direction: FlexDirection::Row,
        flex: 1,
        name: None,
        path: ".".to_string(),
        style: None,
        commands,
        script,
        panes: Vec::new(),
        zoom: false,
        focus: false,
    };

    let session = Session {
        name: "render".to_string(),
        path: ".".to_string(),
        startup: Vec::new(),
        startup_script: None,
        shutdown: Vec::new(),
        shutdown_script: None,
        env: HashMap::new(),
        shell: None,
        pane_cmd_delay: None,
        windows: vec![Window {
            name: "main".to_string(),
            path: None,
            flex_direction: FlexDirection::Row,
            panes: vec![pane],
            focus: false,
        }],
    };

    Ok(session.as_kdl("/tmp", shell)?.to_string())
}

#[test]
fn layout_renders_commands_through_the_session_shell() -> Result<()> {
    let kdl = render_session(
        vec![Command {
            command: "^agent".to_string(),
            args: vec![
                noyalib::compat::serde_yaml::Value::String("a2a".to_string()),
                noyalib::compat::serde_yaml::Value::String("--agent".to_string()),
                noyalib::compat::serde_yaml::Value::String("orchestrator".to_string()),
            ],
        }],
        None,
        "nu",
    )?;

    assert!(
        kdl.contains("command=\"nu\""),
        "the shell wrapper should be rendered: {kdl}"
    );
    assert!(
        kdl.contains("args \"-c\" \"^agent a2a --agent orchestrator\""),
        "the full command line should be rendered: {kdl}"
    );
    assert!(
        !kdl.contains("\\\"--agent\\\""),
        "arguments must not be YAML-quoted: {kdl}"
    );
    Ok(())
}

#[test]
fn layout_renders_env_reference_verbatim() -> Result<()> {
    let kdl = render_session(
        vec![Command {
            command: "^$env.EDITOR".to_string(),
            args: vec![],
        }],
        None,
        "sh",
    )?;

    assert!(
        kdl.contains("^$env.EDITOR"),
        "env references must pass through untouched: {kdl}"
    );
    Ok(())
}

#[test]
fn layout_joins_multiple_commands_with_semicolon() -> Result<()> {
    let kdl = render_session(
        vec![
            Command::from_string("echo one"),
            Command::from_string("echo two"),
        ],
        None,
        "sh",
    )?;

    assert!(
        kdl.contains("args \"-c\" \"echo one; echo two\""),
        "commands should be joined with a semicolon: {kdl}"
    );
    Ok(())
}

#[test]
fn layout_renders_script_only_pane() -> Result<()> {
    let script: Script = noyalib::compat::serde_yaml::from_str("echo hi").into_diagnostic()?;
    let script_command = script.to_cmd()?.to_string();

    let kdl = render_session(Vec::new(), Some(script), "sh")?;

    assert!(
        kdl.contains(&format!("args \"-c\" \"{script_command}\"")),
        "a script-only pane should render its command: {kdl}"
    );
    Ok(())
}

#[test]
fn resolve_shell_prefers_the_explicit_value() {
    assert_eq!(
        resolve_shell(Some("nu".to_string()), Some("fish".to_string())),
        "nu"
    );
}

#[test]
fn resolve_shell_uses_the_explicit_value_without_env() {
    assert_eq!(resolve_shell(Some("nu".to_string()), None), "nu");
}

#[test]
fn resolve_shell_falls_back_to_the_env_value() {
    assert_eq!(resolve_shell(None, Some("fish".to_string())), "fish");
}

#[test]
fn resolve_shell_defaults_to_sh() {
    assert_eq!(resolve_shell(None, None), "sh");
}

#[test]
fn getenv_targets_the_session_and_runs_without_quotes() -> Result<()> {
    let expected_arg = format!(
        "printenv LAIO_CONFIG > {}",
        std::env::temp_dir().join("valid.tmp").display()
    );

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "printenv ZELLIJ"))
        .returning(|_| Ok(String::new()));

    let expected_run_arg = expected_arg.clone();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(move |cmd| {
            let Type::Basic(inner) = cmd else {
                return false;
            };
            let args: Vec<String> = inner
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect();

            args.first().is_some_and(|arg| arg == "run")
                && args.iter().any(|arg| arg == "--block-until-exit")
                && args.iter().any(|arg| arg == &expected_run_arg)
                && args.iter().all(|arg| !arg.contains('"'))
                && inner.get_envs().any(|(key, value)| {
                    key == "ZELLIJ_SESSION_NAME" && value == Some(std::ffi::OsStr::new("valid"))
                })
        })
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string().starts_with("cat ")))
        .returning(|_| Ok("/tmp/laio.yaml".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let client = ZellijClient::new(Rc::new(runner));

    let value = client.getenv("valid", "LAIO_CONFIG")?;

    assert_eq!(value, "/tmp/laio.yaml");
    assert!(
        !expected_arg.contains('"'),
        "the sh -c argument must not embed quotes: {expected_arg}"
    );
    Ok(())
}

fn minimal_session(name: &str) -> Session {
    let pane = Pane {
        flex_direction: FlexDirection::Row,
        flex: 1,
        name: None,
        path: ".".to_string(),
        style: None,
        commands: Vec::new(),
        script: None,
        panes: Vec::new(),
        zoom: false,
        focus: false,
    };

    Session {
        name: name.to_string(),
        path: ".".to_string(),
        startup: Vec::new(),
        startup_script: None,
        shutdown: Vec::new(),
        shutdown_script: None,
        env: HashMap::new(),
        shell: None,
        pane_cmd_delay: None,
        windows: vec![Window {
            name: "main".to_string(),
            path: None,
            flex_direction: FlexDirection::Row,
            panes: vec![pane],
            focus: false,
        }],
    }
}

#[test]
fn mux_start_reports_zellij_stderr_when_the_session_never_appears() -> Result<()> {
    let session = minimal_session("broken");

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Forget(_) if cmd.to_string().contains("nohup zellij --session broken --new-session-with-layout")))
        .returning(|_| Ok(()));

    cmd_bool
        .expect_run()
        .times(3)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string().contains("list-sessions")))
        .returning(|_| Ok(false));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string().starts_with("cat ")))
        .returning(|_| Ok("Failed to parse Zellij configuration".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner_and_poll(runner, 2, Duration::from_millis(10));

    let error = zellij
        .start(&session, &[], false, true)
        .err()
        .ok_or_else(|| miette::miette!("a missing session should return Err"))?;

    assert!(
        error
            .to_string()
            .contains("Failed to parse Zellij configuration"),
        "the error should carry zellij's stderr: {error}"
    );

    let mut layout_path = std::env::temp_dir();
    layout_path.push("broken.kdl");
    assert!(
        !layout_path.exists(),
        "the layout temp file should be deleted on failure: {}",
        layout_path.display()
    );

    let mut error_path = std::env::temp_dir();
    error_path.push("broken.zellij.err");
    assert!(
        !error_path.exists(),
        "the stderr temp file should be deleted on failure: {}",
        error_path.display()
    );

    Ok(())
}
