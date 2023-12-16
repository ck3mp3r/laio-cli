use anyhow::{bail, Result};
use std::process::Command;

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &String) -> Result<T>;
}

#[derive(Clone, Debug)]
pub(crate) struct SystemCmdRunner;

impl Cmd<()> for SystemCmdRunner {
    fn run(&self, cmd: &String) -> Result<()> {
        log::debug!("{}", cmd);
        if Command::new("sh").arg("-c").arg(cmd).status()?.success() {
            Ok(())
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<String> for SystemCmdRunner {
    fn run(&self, cmd: &String) -> Result<String> {
        log::debug!("{}", cmd);
        let output = Command::new("sh").arg("-c").arg(cmd).output()?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            bail!(
                "Command failed: {}\nError: {}",
                cmd,
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }
}

impl Cmd<bool> for SystemCmdRunner {
    fn run(&self, cmd: &String) -> Result<bool> {
        log::debug!("{}", cmd);
        let output = Command::new("sh").arg("-c").arg(cmd).output()?;
        if output.status.success() {
            Ok(true)
        } else {
            bail!(
                "Command failed: {}\nError: {}",
                cmd,
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }
}

impl SystemCmdRunner {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

pub(crate) trait CmdRunner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

impl CmdRunner for SystemCmdRunner {}

#[cfg(test)]
pub mod test {
    use std::{cell::RefCell, sync::Mutex};

    use super::{Cmd, CmdRunner};
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
    // Mock implementation for testing purposes
    #[derive(Clone, Debug)]
    pub(crate) struct MockCmdRunner {
        cmds: RefCell<Vec<String>>,
    }

    impl MockCmdRunner {
        pub fn new() -> Self {
            Self {
                cmds: RefCell::new(vec![]),
            }
        }

        pub fn push(&self, cmd: String) {
            self.cmds.borrow_mut().push(cmd);
        }

        pub fn get_cmds(&self) -> Vec<String> {
            self.cmds.borrow().clone()
        }

        pub fn cmds(&self) -> &RefCell<Vec<String>> {
            &self.cmds
        }
    }

    impl Cmd<()> for MockCmdRunner {
        fn run(&self, cmd: &String) -> Result<(), anyhow::Error> {
            debug!("{}", cmd);
            self.push(cmd.clone());
            Ok(())
        }
    }

    impl Cmd<String> for MockCmdRunner {
        fn run(&self, cmd: &String) -> Result<String, anyhow::Error> {
            debug!("{}", cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\"" => {
                    Ok("width: 160\nheight: 90".to_string())
                }
                "tmux new-window -Pd -t test -n code -c /tmp -F \"#{window_id}\"" => {
                    Ok(format!("@{}", next_window_id()))
                }
                "tmux new-window -Pd -t test -n infrastructure -c /tmp -F \"#{window_id}\"" => {
                    Ok(format!("@{}", next_window_id()))
                }
                "tmux split-window -t test:@1 -c /tmp -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t test:@1 -c /tmp/src -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t test:@2 -c /tmp/one -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t test:@2 -c /tmp/two -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux split-window -t test:@2 -c /tmp/three -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", next_pane_id()))
                }
                "tmux display-message -t test:@1 -p \"#P\"" => Ok(format!("%{}", next_pane_id())),
                "tmux display-message -t test:@2 -p \"#P\"" => Ok(format!("%{}", next_pane_id())),
                "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\"" => Ok(
                    "code ce5e,274x86,0,0,1\nmisc 6b9f,274x86,0,0{137x86,0,0[137x27,0,0{42x27,0,0,2,46x27,43,0,6,47x27,90,0,8},137x58,0,28,4],136x86,138,0[136x43,138,0,5,136x21,138,44,10,136x20,138,66{86x20,138,66,11,49x20,225,66,12}]}".to_string(),
                ),
                "printenv TMUX" => Ok("foo".to_string()),
                "tmux show-options -g base-index" => Ok("base-index 1".to_string()),
                "tmux ls -F \"#{session_name}\"" => Ok(format!("{}","foo\nbar")),
                "tmux show-environment -t test: LAIO_CONFIG" => Ok("LAIO_CONFIG=./src/commands/session/test/test.yaml".to_string()),
                _ => {
                    println!("{}", cmd);
                    Ok("".to_string())
                },
            }
        }
    }

    impl Cmd<bool> for MockCmdRunner {
        fn run(&self, cmd: &String) -> Result<bool, anyhow::Error> {
            debug!("{}", cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux has-session -t test" => Ok(false),
                _ => Ok(true),
            }
        }
    }

    impl CmdRunner for MockCmdRunner {}
}
