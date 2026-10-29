use std::{fs, time::Duration};

use miette::{IntoDiagnostic, Result, miette};

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use super::client::{HerdrApi, HerdrClient};

use super::support_env::*;
use super::support_responses::*;
use super::support_socket::*;

#[test]
fn ensure_server_returns_ok_without_spawning_when_ping_succeeds() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![pong_response()]);

    session.client().ensure_server(session.name())?;

    let requests = join_thread(handle)?;
    assert_eq!(requests.len(), 1);
    Ok(())
}

#[test]
fn ensure_server_spawns_server_with_session_env_and_polls() -> Result<()> {
    let session = TestSession::new()?;
    let socket_path = session.socket_path()?;

    let mut cmd_unit = MockCmdUnitMock::new();
    let path = socket_path.clone();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=w sh -c nohup herdr server >/dev/null 2>&1 </dev/null & disown")
        })
        .returning(move |_| {
            let parent = path
                .parent()
                .ok_or_else(|| miette!("socket path should have a parent directory"))?;
            fs::create_dir_all(parent).into_diagnostic()?;
            let listener = bind_listener(&path)?;
            let _ = spawn_canned_server(listener, vec![pong_response()]);
            Ok(())
        });

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let client = HerdrClient::new_with_runner(runner);

    client.ensure_server(session.name())?;
    Ok(())
}

#[test]
fn ensure_server_times_out_naming_the_socket_path() -> Result<()> {
    let session = TestSession::new()?;
    let socket_path = session.socket_path()?;

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit.expect_run().times(1).returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let client = HerdrClient::new_with_runner(runner);

    let error = client
        .ensure_server_with_timeout(session.name(), Duration::from_millis(100))
        .err()
        .ok_or_else(|| miette!("missing server should return Err"))?;

    assert!(
        error
            .to_string()
            .contains(socket_path.to_string_lossy().as_ref()),
        "report should name the socket path: {error}"
    );
    Ok(())
}

#[test]
fn session_exists_returns_true_when_socket_connects() -> Result<()> {
    let session = TestSession::new()?;
    let client = session.client();

    assert!(!client.session_exists(session.name()));

    let listener = session.bind()?;
    assert!(client.session_exists(session.name()));

    drop(listener);
    Ok(())
}
