use crate::common::cmd::test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock};
use crate::muxer::tmux::Target;
use crate::tmux_target;
use miette::Result;
use std::{collections::HashMap, sync::Arc};

use super::client::TmuxClient;

#[test]
fn client_create_session() -> Result<()> {
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            let temp_dir = std::env::temp_dir();
            cmd.to_string()
                == format!(
                    "tmux new-session -d -s test -c {}",
                    temp_dir.to_string_lossy()
                )
        })
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string() == "tmux set-option -t test default-shell /bin/zsh")
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string().contains("new-window"))
        .returning(|_| Ok("test_window_id".to_string()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string().contains("select-layout"))
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux_client = TmuxClient::new(Arc::new(runner));
    let session_name = "test";

    let temp_dir = std::env::temp_dir();
    let temp_dir_str = temp_dir.to_string_lossy();
    tmux_client.create_session(
        &String::from("test"),
        temp_dir_str.as_ref(),
        &HashMap::new(),
        &Some("/bin/zsh".to_string()),
    )?;
    tmux_client.new_window(session_name, "test", &temp_dir_str)?;
    tmux_client.select_layout(&tmux_target!(session_name, "@1"), "main-horizontal")?;
    Ok(())
}

#[test]
fn client_register_command_batches_commands() -> Result<()> {
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    // Expect a single send-keys call with multiple commands interleaved with C-m
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            cmd.to_string()
                == "tmux send-keys -t test:@1.%1 echo hello C-m echo world C-m ls -la C-m"
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux_client = TmuxClient::new(Arc::new(runner));
    let target = tmux_target!("test", "@1", "%1");

    // Register multiple commands at once
    tmux_client.register_command(
        &target,
        vec![
            "echo hello".to_string(),
            "echo world".to_string(),
            "ls -la".to_string(),
        ],
    );

    // Flush commands to execute (now synchronous!)
    tmux_client.flush_commands();

    Ok(())
}

#[test]
fn client_register_command_handles_empty_vec() -> Result<()> {
    let cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    // Should NOT call run when commands vec is empty
    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux_client = TmuxClient::new(Arc::new(runner));
    let target = tmux_target!("test", "@1", "%1");

    // Register empty command list
    tmux_client.register_command(&target, vec![]);

    // Flush commands - should not panic or call any mock
    tmux_client.flush_commands();

    Ok(())
}
