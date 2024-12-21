use std::rc::Rc;

use crate::{
    common::cmd::test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
    driver::tmux::{Client, Target},
};

#[test]
fn new_session() -> Result<(), anyhow::Error> {
    // Create mocks for the RunnerMock components
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    // Set up expectations for each mock as necessary
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string() == "tmux new-session -d -s \"test\" -c \"/tmp\"")
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

    // Construct the RunnerMock with the mocks
    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    // Create the Client instance with the mock runner
    let tmux = Client::new(Rc::new(runner));
    let session_name = "test";

    // Execute the tmux commands
    tmux.create_session(&String::from("test"), &String::from("/tmp"))?;
    tmux.new_window(session_name, "test", "/tmp")?;
    tmux.select_layout(
        &Target::new(session_name).window("@1"),
        "main-horizontal",
    )?;

    // Note: Verifying the command history or calls now relies on the mocks
    Ok(())
}
