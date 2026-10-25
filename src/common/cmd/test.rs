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

impl Type {
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        match self {
            Type::Basic(cmd) | Type::Verbose(cmd) | Type::Forget(cmd) => {
                let envs: Vec<_> = cmd
                    .get_envs()
                    .filter_map(|(key, value)| {
                        value.map(|v| format!("{}={}", key.to_string_lossy(), v.to_string_lossy()))
                    })
                    .collect();
                let args: Vec<_> = cmd.get_args().map(|arg| arg.to_string_lossy()).collect();
                let cmd_str = if args.is_empty() {
                    cmd.get_program().to_string_lossy().to_string()
                } else {
                    format!("{} {}", cmd.get_program().to_string_lossy(), args.join(" "))
                };
                if envs.is_empty() {
                    cmd_str
                } else {
                    format!("{} {}", envs.join(" "), cmd_str)
                }
            }
        }
    }
}
