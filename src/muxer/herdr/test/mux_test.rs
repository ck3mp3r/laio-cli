use std::collections::HashMap;

use miette::{Result, miette};
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;
use crate::common::session_info::{SessionInfo, SessionStatus};

use super::super::model::WorkspaceInfo;
use super::super::mux::{Herdr, LAIO_SESSION_NAME};

use super::*;

#[test]
fn list_sessions_maps_owned_workspaces_to_active() -> Result<()> {
    let _guard = env_lock()?;
    let previous_workspace = clear_herdr_workspace_id();

    let output = json!({
        "sessions": [
            {"name": "default", "default": true, "running": true},
            {"name": "laio", "default": false, "running": true},
            {"name": "stopped", "default": false, "running": false}
        ]
    })
    .to_string();

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(output.clone()));

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w1", "alpha", laio_tokens()),
                workspace_info_with("w2", "beta", HashMap::new()),
                workspace_info_with("w3", "gamma", laio_tokens()),
            ])
        });

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let sessions = herdr.list_sessions();
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let sessions = sessions?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].name, "alpha");
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].name, "gamma");
    assert_eq!(sessions[1].status, SessionStatus::Active);
    Ok(())
}

#[test]
fn list_sessions_skips_duplicate_workspaces() -> Result<()> {
    let _guard = env_lock()?;
    let previous_workspace = clear_herdr_workspace_id();

    let output = json!({
        "sessions": [
            {"name": "laio", "default": false, "running": true},
            {"name": "other", "default": false, "running": true}
        ]
    })
    .to_string();

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(output.clone()));

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "alpha", laio_tokens())]));
    api.expect_workspace_list()
        .withf(|session| session == "other")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "alpha", laio_tokens())]));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let sessions = herdr.list_sessions();
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let sessions = sessions?;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "alpha");
    Ok(())
}

fn list_sessions_inventory() -> Vec<WorkspaceInfo> {
    vec![
        workspace_info_with("w1", "alpha", laio_tokens()),
        workspace_info_with("w2", "beta", laio_tokens()),
        workspace_info_with("w3", "gamma", HashMap::new()),
    ]
}

fn list_sessions_with_workspace_env(workspace_id: Option<&str>) -> Result<Vec<SessionInfo>> {
    let output =
        json!({"sessions": [{"name": "laio", "default": false, "running": true}]}).to_string();

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(output.clone()));

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| Ok(list_sessions_inventory()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let previous = match workspace_id {
        Some(workspace_id) => set_herdr_workspace_id(workspace_id),
        None => clear_herdr_workspace_id(),
    };
    let sessions = herdr.list_sessions();
    restore_env("HERDR_WORKSPACE_ID", previous);

    sessions
}

#[test]
fn list_sessions_marks_the_current_workspace_attached() -> Result<()> {
    let _guard = env_lock()?;

    let sessions = list_sessions_with_workspace_env(Some("w2"))?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].name, "alpha");
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].name, "beta");
    assert_eq!(sessions[1].status, SessionStatus::Attached);
    Ok(())
}

#[test]
fn list_sessions_marks_every_row_active_outside_herdr() -> Result<()> {
    let _guard = env_lock()?;

    let sessions = list_sessions_with_workspace_env(None)?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].status, SessionStatus::Active);
    Ok(())
}

#[test]
fn list_sessions_marks_every_row_active_when_the_workspace_is_absent() -> Result<()> {
    let _guard = env_lock()?;

    let sessions = list_sessions_with_workspace_env(Some("w9"))?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].status, SessionStatus::Active);
    Ok(())
}

#[test]
fn target_session_uses_the_socket_path_session_name() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_socket_path("/tmp/herdr/sessions/work/herdr.sock");

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let target = herdr.target_session();
    restore_env("HERDR_SOCKET_PATH", previous);

    assert_eq!(target, "work");
    Ok(())
}

#[test]
fn target_session_falls_back_to_the_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_socket_path();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let target = herdr.target_session();
    restore_env("HERDR_SOCKET_PATH", previous);

    assert_eq!(target, LAIO_SESSION_NAME);
    Ok(())
}

#[test]
fn target_session_falls_back_when_the_socket_path_does_not_match() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_socket_path("/tmp/herdr/herdr.sock");

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let target = herdr.target_session();
    restore_env("HERDR_SOCKET_PATH", previous);

    assert_eq!(target, LAIO_SESSION_NAME);
    Ok(())
}

#[test]
fn current_workspace_id_reads_the_env_var() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_workspace_id("w3");

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let workspace_id = herdr.current_workspace_id();
    restore_env("HERDR_WORKSPACE_ID", previous);

    assert_eq!(workspace_id.as_deref(), Some("w3"));
    Ok(())
}

#[test]
fn current_workspace_id_ignores_empty_and_unset_values() -> Result<()> {
    let _guard = env_lock()?;
    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let previous = set_herdr_workspace_id("");
    let empty = herdr.current_workspace_id();
    restore_env("HERDR_WORKSPACE_ID", previous);

    let previous = clear_herdr_workspace_id();
    let unset = herdr.current_workspace_id();
    restore_env("HERDR_WORKSPACE_ID", previous);

    assert_eq!(empty, None);
    assert_eq!(unset, None);
    Ok(())
}

#[test]
fn owned_workspaces_keeps_only_token_carrying_workspaces() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "alpha", laio_tokens()),
            workspace_info_with("w2", "beta", HashMap::new()),
            workspace_info_with("w3", "gamma", laio_tokens()),
        ])
    });

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let owned = herdr.owned_workspaces(LAIO_SESSION_NAME)?;

    assert_eq!(owned.len(), 2);
    assert_eq!(owned[0].label, "alpha");
    assert_eq!(owned[1].label, "gamma");
    Ok(())
}

#[test]
fn find_owned_workspace_matches_the_label() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "alpha", laio_tokens()),
            workspace_info_with("w2", "beta", laio_tokens()),
        ])
    });

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let found = herdr.find_owned_workspace(LAIO_SESSION_NAME, "beta")?;

    let workspace = found.ok_or_else(|| miette!("beta should be found"))?;
    assert_eq!(workspace.workspace_id, "w2");
    Ok(())
}

#[test]
fn find_owned_workspace_ignores_workspaces_without_tokens() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "beta", HashMap::new())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    assert!(
        herdr
            .find_owned_workspace(LAIO_SESSION_NAME, "beta")?
            .is_none()
    );
    Ok(())
}

#[test]
fn unimplemented_multiplexer_methods_return_err() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous);

    assert!(result.is_err());
    Ok(())
}

#[test]
fn herdr_new_rejects_invalid_session_names() -> Result<()> {
    let herdr = Herdr::new();

    let error = herdr
        .switch("", true)
        .err()
        .ok_or_else(|| miette!("empty session name should return Err"))?;

    assert!(
        error.to_string().contains("Invalid"),
        "report should name the invalid session: {error}"
    );
    Ok(())
}
