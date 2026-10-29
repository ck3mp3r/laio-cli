use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};
use crate::common::muxer::multiplexer::Multiplexer;
use crate::common::session_info::SessionStatus;
use crate::muxer::Tmux;
use miette::Result;

#[test]
fn mux_stop_session() -> Result<()> {
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_bool
        .expect_run()
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux has-session -t valid"),
        )
        .times(2)
        .returning(|_| Ok(true));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c [ -n \"$TMUX\" ] && tmux display-message -p '#S' || true"))
        .times(1)
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux show-environment -t valid LAIO_CONFIG"))
        .times(2)
        .returning(|_| {
            Ok(format!(
                "LAIO_CONFIG={}/src/common/config/fixtures/valid.yaml",
                env!("CARGO_MANIFEST_DIR")
            ))
        });

    // Mock LAIO_VARS retrieval (returns empty for backward compatibility test)
    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux show-environment -t valid LAIO_VARS"))
        .times(1)
        .returning(|_| Err(miette::miette!("Not found"))); // Simulate no LAIO_VARS (old session)

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "date"))
        .times(1)
        .returning(|_| Ok("something".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo Bye"))
        .times(1)
        .returning(|_| Ok("Bye".to_string()));

    cmd_unit
        .expect_run()
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux kill-session -t valid"),
        )
        .times(1)
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.stop(&Some("valid".to_string()), &None, false, false, false);
    eprintln!("{:?}", result);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn mux_list_sessions() -> Result<()> {
    let cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux ls -F #{session_name}|#{session_attached}"),
        )
        .times(1)
        .returning(|_| Ok("foo|1\nbar|0\nbaz|1\n".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.list_sessions()?;

    let sessions = result;
    assert_eq!(sessions.len(), 3);
    assert_eq!(sessions[0].name, "foo");
    assert_eq!(sessions[0].status, SessionStatus::Attached);
    assert_eq!(sessions[1].name, "bar");
    assert_eq!(sessions[1].status, SessionStatus::Active);
    assert_eq!(sessions[2].name, "baz");
    assert_eq!(sessions[2].status, SessionStatus::Attached);

    Ok(())
}
