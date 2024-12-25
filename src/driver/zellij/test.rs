use crate::common::{
    cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
    config::Session,
    mux::Multiplexer,
    path::{current_working_path, to_absolute_path},
};
use anyhow::Result;

use super::Zellij;

#[test]
fn client_create_session() -> Result<()> {
    let cwd = current_working_path().expect("Cannot get current working directory");
    let path = to_absolute_path(&format!(
        "{}/src/common/config/test/valid.yaml",
        cwd.to_string_lossy()
    ))
    .unwrap();

    let session = Session::from_config(&path).unwrap();
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
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

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    let _result = zellij.start(&session, &path.to_string_lossy(), false, false)?;

    Ok(())
}
