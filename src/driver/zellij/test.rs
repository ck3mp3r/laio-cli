use crate::common::{
    cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
    config::Session,
    mux::Multiplexer,
    path::to_absolute_path,
};
use anyhow::Result;

use super::Zellij;

#[test]
fn mux_start_session() -> Result<()> {
    let path = to_absolute_path("./src/common/config/test/valid.yaml").unwrap();

    let session = Session::from_config(&path).unwrap();
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Forget(content) if content == "zellij --session valid --new-session-with-layout /tmp/valid.kdl"))
        .returning(|_| Ok(()));

    cmd_bool
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij list-sessions | grep \"valid\""))
        .returning(|_| Ok(false));

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Verbose(content) if vec!["date", "echo Hi"].contains(&content.as_str())),
        )
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    let _result = zellij.start(&session, &path.to_string_lossy(), false, false)?;

    Ok(())
}
#[test]
fn mux_stop_session() -> Result<()> {
    // let path = to_absolute_path("./src/common/config/test/valid.yaml").unwrap();

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(content) if content == "printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("".to_string()));

    cmd_bool
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij list-sessions | grep \"valid\""))
        .returning(|_| Ok(true));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij delete-session \"valid\" --force"))
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    let _result = zellij.stop(&Some("valid".to_string()), false, false)?;

    Ok(())
}
