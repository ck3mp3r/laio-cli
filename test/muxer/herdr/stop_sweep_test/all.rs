use std::collections::HashMap;

use miette::Result;
use mockall::Sequence;
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;

use super::mux::Herdr;

use super::support_api_mock::*;
use super::support_env::*;
use super::support_models::*;

#[test]
fn stop_all_closes_every_owned_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_env("1");

    let sessions = json!({
        "sessions": [
            {"name": "current", "running": true},
            {"name": "mine", "running": true},
            {"name": "other", "running": true}
        ]
    })
    .to_string();

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|name| name == "current")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-current",
                "current",
                laio_tokens(),
            )])
        });
    api.expect_workspace_list()
        .withf(|name| name == "mine")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w-mine", "mine", laio_tokens())]));
    api.expect_workspace_list()
        .withf(|name| name == "other")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-other",
                "other",
                HashMap::new(),
            )])
        });
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "current" && workspace_id == "w-current")
        .returning(|_, _| Ok(()));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "mine" && workspace_id == "w-mine")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .withf(|name| name == "laio")
        .times(1)
        .returning(|_| false);

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn stop_all_tears_down_only_the_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({
        "sessions": [
            {"name": "laio", "running": true},
            {"name": "work", "running": true}
        ]
    })
    .to_string();

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
        .withf(|name| name == "laio")
        .returning(|_| Ok(vec![workspace_info_with("w-laio", "alpha", laio_tokens())]));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|name| name == "work")
        .returning(|_| Ok(vec![workspace_info_with("w-work", "beta", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-laio")
        .returning(|_, _| Ok(()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "work" && workspace_id == "w-work")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|name| name == "laio")
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

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_all_tears_down_an_empty_laio_session_with_no_owned_workspaces() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

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
        .returning(|_| Ok(Vec::new()));
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

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_all_tears_down_the_laio_session_when_a_tokenless_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "alpha", laio_tokens()),
            workspace_info_with("w2", "user", HashMap::new()),
        ])
    });
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
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

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_all_tears_down_unconditionally_when_a_token_carrying_workspace_survives() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "alpha", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}
