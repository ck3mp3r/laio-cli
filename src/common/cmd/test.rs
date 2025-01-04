use super::{Cmd, Runner, Type};
use miette::Result;
use mockall::mock;

mock! {
    pub CmdUnitMock {}

    impl Cmd<()> for CmdUnitMock {
        fn run(&self, cmd: &Type) -> Result<()>;
    }
}

mock! {
    pub CmdStringMock {}

    impl Cmd<String> for CmdStringMock {
        fn run(&self, cmd: &Type) -> Result<String>;
    }
}

mock! {
    pub CmdBoolMock {}

    impl Cmd<bool> for CmdBoolMock {
        fn run(&self, cmd: &Type) -> Result<bool>;
    }
}

pub struct RunnerMock {
    pub cmd_unit: MockCmdUnitMock,
    pub cmd_string: MockCmdStringMock,
    pub cmd_bool: MockCmdBoolMock,
}

impl Clone for RunnerMock {
    fn clone(&self) -> Self {
        Self {
            cmd_unit: MockCmdUnitMock::new(),
            cmd_string: MockCmdStringMock::new(),
            cmd_bool: MockCmdBoolMock::new(),
        }
    }
}

impl Runner for RunnerMock {}

impl Cmd<()> for RunnerMock {
    fn run(&self, cmd: &Type) -> Result<()> {
        self.cmd_unit.run(cmd)
    }
}

impl Cmd<String> for RunnerMock {
    fn run(&self, cmd: &Type) -> Result<String> {
        self.cmd_string.run(cmd)
    }
}

impl Cmd<bool> for RunnerMock {
    fn run(&self, cmd: &Type) -> Result<bool> {
        self.cmd_bool.run(cmd)
    }
}
