use anyhow::{bail, Result};
use std::{
    fmt,
    io::BufRead,
    io::BufReader,
    io::Write,
    process::{Command, ExitStatus, Stdio},
};

#[derive(Clone, Debug, PartialEq)]
pub enum CommandType {
    Basic(String),
    Verbose(String),
    Forget(String),
}

impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandType::Basic(cmd) | CommandType::Verbose(cmd) => write!(f, "{}", cmd),
            CommandType::Forget(cmd) => write!(f, "{}", cmd),
        }
    }
}

#[macro_export]
macro_rules! cmd_basic {
    ($($arg:tt)*) => {
        CommandType::Basic(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_verbose {
    ($($arg:tt)*) => {
        CommandType::Verbose(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_forget {
    ($($arg:tt)*) => {
        CommandType::Forget(format!($($arg)*))
    };
}

const PROMPT_CHAR: &str = "❯";

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &CommandType) -> Result<T>;
}

#[derive(Clone, Debug)]
pub(crate) struct SystemCmdRunner;

impl Cmd<()> for SystemCmdRunner {
    fn run(&self, cmd: &CommandType) -> Result<()> {
        let (_, status) = self.run(&cmd)?;

        if status.success() {
            Ok(())
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<String> for SystemCmdRunner {
    fn run(&self, cmd: &CommandType) -> Result<String> {
        let (output, status) = self.run(&cmd)?;

        if status.success() {
            Ok(output)
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<bool> for SystemCmdRunner {
    fn run(&self, cmd: &CommandType) -> Result<bool> {
        let (_, status) = self.run(&cmd)?;

        Ok(status.success())
    }
}

impl SystemCmdRunner {
    pub(crate) fn new() -> Self {
        Self {}
    }

    fn run(&self, cmd: &CommandType) -> Result<(String, ExitStatus)> {
        let (command_string, is_verbose, should_wait) = match cmd {
            CommandType::Basic(c) => (c, false, true),
            CommandType::Verbose(c) => (c, true, true),
            CommandType::Forget(c) => (c, true, false),
        };

        log::debug!("{}", &command_string);

        if !should_wait {
            let status = Command::new("sh").arg("-c").arg(&command_string).status()?;
            return Ok((String::new(), status));
        }

        if is_verbose {
            println!("{} {}", &PROMPT_CHAR, &command_string);
        }

        let mut command = Command::new("sh")
            .arg("-c")
            .arg(&command_string)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut buffer = Vec::new();
        if let Some(o) = command.stdout.take() {
            let reader = BufReader::new(o);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if is_verbose {
                            println!("{}", line);
                        }
                        writeln!(buffer, "{}", line)?;
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }

        let status = command.wait()?;
        let output = String::from_utf8(buffer)?;
        Ok((output.trim().to_string(), status))
    }
}

pub(crate) trait CmdRunner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

impl CmdRunner for SystemCmdRunner {}

#[cfg(test)]
pub mod test {
    use std::{cell::RefCell, sync::Mutex};

    use super::{Cmd, CmdRunner, CommandType};
    use lazy_static::lazy_static;
    use log::debug;

    lazy_static! {
        static ref WINDOW_NUMBER_GENERATOR: Mutex<i32> = Mutex::new(0);
        static ref PANE_NUMBER_GENERATOR: Mutex<i32> = Mutex::new(0);
    }

    fn next_window_id() -> i32 {
        let mut num = WINDOW_NUMBER_GENERATOR.lock().unwrap();
        *num += 1;
        *num
    }
    fn next_pane_id() -> i32 {
        let mut num = PANE_NUMBER_GENERATOR.lock().unwrap();
        *num += 1;
        *num
    }

    impl CommandType {
        // Method to get a string slice from CommandType
        pub fn as_str(&self) -> &str {
            match self {
                CommandType::Basic(cmd) | CommandType::Verbose(cmd) => cmd.as_str(),
                CommandType::Forget(cmd) => cmd.as_str(),
            }
        }
    }

    // Mock implementation for testing purposes
    #[derive(Clone, Debug)]
    pub(crate) struct MockCmdRunner {
        cmds: RefCell<Vec<CommandType>>,
    }

    impl MockCmdRunner {
        pub fn new() -> Self {
            Self {
                cmds: RefCell::new(vec![]),
            }
        }

        pub fn push(&self, cmd: CommandType) {
            self.cmds.borrow_mut().push(cmd);
        }

        pub fn get_cmds(&self) -> Vec<CommandType> {
            self.cmds.borrow().clone()
        }

        pub fn cmds(&self) -> &RefCell<Vec<CommandType>> {
            &self.cmds
        }
    }

    impl Cmd<()> for MockCmdRunner {
        fn run(&self, cmd: &CommandType) -> Result<(), anyhow::Error> {
            debug!("{:?}", cmd);
            self.push(cmd.clone());
            Ok(())
        }
    }

    impl Cmd<String> for MockCmdRunner {
        fn run(&self, cmd: &CommandType) -> Result<String, anyhow::Error> {
            debug!("{}", cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\"" => {
                    Ok("width: 160\nheight: 90".to_string())
                }
                "tmux new-window -Pd -t valid -n code -c /tmp -F \"#{window_id}\"" => {
                    Ok(format!("@{}", next_window_id()))
                }
                "tmux new-window -Pd -t valid -n infrastructure -c /tmp -F \"#{window_id}\"" => {
                    Ok(format!("@{}", next_window_id()))
                }
                "tmux split-window -t valid:@1 -c /tmp -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t valid:@1 -c /tmp/src -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t valid:@2 -c /tmp/one -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t valid:@2 -c /tmp/two -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t valid:@2 -c /tmp/three -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux display-message -t valid:@1 -p \"#P\"" => Ok(format!("%{}", next_pane_id())),
                "tmux display-message -t valid:@2 -p \"#P\"" => Ok(format!("%{}", next_pane_id())),
                "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\"" => Ok(
                    "code ce5e,274x86,0,0,1\nmisc 6b9f,274x86,0,0{137x86,0,0[137x27,0,0{42x27,0,0,2,46x27,43,0,6,47x27,90,0,8},137x58,0,28,4],136x86,138,0[136x43,138,0,5,136x21,138,44,10,136x20,138,66{86x20,138,66,11,49x20,225,66,12}]}".to_string(),
                ),
                "printenv TMUX" => Ok("foo".to_string()),
                "tmux show-options -g base-index" => Ok("base-index 1".to_string()),
                "tmux ls -F \"#{session_name}\"" => Ok(format!("{}","foo\nbar")),
                "tmux show-environment -t valid: LAIO_CONFIG" => Ok("LAIO_CONFIG=./src/app/manager/test/valid.yaml".to_string()),
                _ => {
                    println!("{}", cmd);
                    Ok("".to_string())
                },
            }
        }
    }

    impl Cmd<bool> for MockCmdRunner {
        fn run(&self, cmd: &CommandType) -> Result<bool, anyhow::Error> {
            debug!("{}", cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux has-session -t valid" => Ok(false),
                _ => Ok(true),
            }
        }
    }

    impl CmdRunner for MockCmdRunner {}
}
