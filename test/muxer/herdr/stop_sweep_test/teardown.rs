use std::{collections::HashMap, env, fs};

use miette::{IntoDiagnostic, Result};
use mockall::Sequence;
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;

use super::mux::{Herdr, LAIO_SESSION_NAME};

use super::support::*;

#[test]
fn stop_sweep_runs_token_loaded_shutdown_hooks_before_closing() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let config_file = env::temp_dir().join(format!("laio-herdr-sweep-{}.yaml", std::process::id()));
    let config = "---\nname: sweep\npath: /tmp\nshutdown:\n  - command: echo\n    args:\n      - sweep-bye\nwindows:\n  - name: w\n    panes:\n      - flex: 1\n";
    fs::write(&config_file, config).into_diagnostic()?;
    let config_path = config_file.to_string_lossy().to_string();

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();
    let tokens = config_tokens(&config_path);

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning({
            let tokens = tokens.clone();
            move |_| Ok(vec![workspace_info_with("w1", "sweep", tokens.clone())])
        });
    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .returning({
            let tokens = tokens.clone();
            move |_, _| Ok(workspace_info_with("w1", "sweep", tokens.clone()))
        });
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo sweep-bye"))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'n=0; until herdr session stop laio >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, true, false);
    restore_env("HERDR_ENV", previous_env);
    let _ = fs::remove_file(&config_file);

    result
}

#[test]
fn stop_sweep_warns_and_continues_without_laio_config() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "sweep", laio_tokens())]));
    api.expect_workspace_get()
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w1", "sweep", HashMap::new())));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'n=0; until herdr session stop laio >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_tears_down_the_laio_session_after_closing_the_last_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_tears_down_the_laio_session_when_a_tokenless_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "work", laio_tokens()),
            workspace_info_with("w2", "user", HashMap::new()),
        ])
    });
    api.expect_workspace_close()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w2", "user", HashMap::new())]));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_never_tears_down_a_foreign_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/work/herdr.sock");

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "work" && workspace_id == "w1")
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_keeps_the_laio_session_when_an_owned_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w3", "late", laio_tokens())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}
