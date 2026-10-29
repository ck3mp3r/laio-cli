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

use super::support::*;

#[test]
fn stop_other_closes_other_workspaces_and_keeps_the_current_one() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-current");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/current/herdr.sock");

    let sessions = json!({
        "sessions": [
            {"name": "current", "running": true},
            {"name": "mine", "running": true}
        ]
    })
    .to_string();

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|name| name == "mine")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-mine",
                "mine",
                config_tokens("/tmp/mine.yaml"),
            )])
        });
    api.expect_workspace_list()
        .withf(|name| name == "current")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-current",
                "current",
                config_tokens("/tmp/current.yaml"),
            )])
        });
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "current" && workspace_id == "w-current")
        .times(1)
        .returning(|_, _| {
            Ok(workspace_info_with(
                "w-current",
                "current",
                config_tokens("/tmp/current.yaml"),
            ))
        });
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "mine" && workspace_id == "w-mine")
        .times(1)
        .returning(|_, _| {
            Ok(workspace_info_with(
                "w-mine",
                "mine",
                config_tokens("/tmp/mine.yaml"),
            ))
        });
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

    let result = herdr.stop(&None, &None, true, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_other_skips_hooks_for_the_current_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-current");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/current/herdr.sock");

    let sessions = json!({"sessions": [{"name": "current", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|name| name == "current")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-current",
                "current",
                config_tokens("/tmp/current.yaml"),
            )])
        });
    api.expect_workspace_get().times(1).returning(|_, _| {
        Ok(workspace_info_with(
            "w-current",
            "current",
            config_tokens("/tmp/current.yaml"),
        ))
    });
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

    let result = herdr.stop(&None, &None, false, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_other_keeps_the_laio_session_when_the_host_workspace_is_kept() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-host");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/laio/herdr.sock");

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
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w-host", "host", config_tokens("/tmp/host.yaml")),
            workspace_info_with("w-other", "other", config_tokens("/tmp/other.yaml")),
        ])
    });
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-host")
        .times(1)
        .returning(|_, _| {
            Ok(workspace_info_with(
                "w-host",
                "host",
                config_tokens("/tmp/host.yaml"),
            ))
        });
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-other")
        .times(1)
        .returning(|_, _| {
            Ok(workspace_info_with(
                "w-other",
                "other",
                config_tokens("/tmp/other.yaml"),
            ))
        });
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-other")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w-host", "host", laio_tokens())]));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_other_tears_down_detached_when_laio_sits_in_a_tokenless_pane() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-tokenless");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/laio/herdr.sock");

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
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-tokenless")
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w-tokenless", "~", HashMap::new())));
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w1")
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w1", "alpha", laio_tokens())));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(Vec::new()));
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

    let result = herdr.stop(&None, &None, true, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_other_keeps_the_laio_session_when_a_token_carrying_workspace_survives() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-mine");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/laio/herdr.sock");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Ok(workspace_info_with("w-mine", "mine", laio_tokens())));
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
        .returning(|_| Ok(vec![workspace_info_with("w-mine", "mine", laio_tokens())]));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w-new", "new", laio_tokens())]));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}
