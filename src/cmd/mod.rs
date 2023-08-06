use std::{error::Error, process::Command};

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &String) -> Result<T, Box<dyn Error>>;
}
#[derive(Clone, Debug)]
pub(crate) struct SystemCmdRunner;

impl Cmd<()> for SystemCmdRunner {
    fn run(&self, cmd: &String) -> Result<(), Box<dyn Error>> {
        #[cfg(debug_assertions)]
        {
            dbg!(cmd);
        }

        let output = Command::new("sh").arg("-c").arg(&cmd).status()?;
        match output.success() {
            true => Ok(()),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Command failed: {}", cmd),
            ))),
        }
    }
}

impl Cmd<String> for SystemCmdRunner {
    fn run(&self, cmd: &String) -> Result<String, Box<dyn Error>> {
        #[cfg(debug_assertions)]
        {
            dbg!(cmd);
        }
        let output = Command::new("sh").arg("-c").arg(&cmd).output()?;
        match output.status.success() {
            true => {
                let stdout = String::from_utf8(output.stdout)?;
                Ok(stdout.trim().to_string())
            }
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8(output.stderr)?,
            ))),
        }
    }
}

impl Cmd<bool> for SystemCmdRunner {
    fn run(&self, cmd: &String) -> Result<bool, Box<dyn Error>> {
        #[cfg(debug_assertions)]
        {
            dbg!(cmd);
        }
        let output = Command::new("sh").arg("-c").arg(&cmd).output()?;
        match output.status.success() {
            true => Ok(true),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8(output.stderr)?,
            ))),
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
    use std::{cell::RefCell, error::Error};

    use super::{Cmd, CmdRunner};

    // Mock implementation for testing purposes
    #[derive(Clone, Debug)]
    pub(crate) struct MockCmdRunner {
        cmds: RefCell<Vec<String>>,
    }

    impl MockCmdRunner {
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self {
                cmds: RefCell::new(vec![]),
            }
        }

        pub fn push(&self, cmd: String) {
            self.cmds.borrow_mut().push(cmd);
        }

        #[allow(dead_code)]
        pub fn get_cmds(&self) -> Vec<String> {
            self.cmds.borrow().clone()
        }

        pub fn cmds(&self) -> &RefCell<Vec<String>> {
            &self.cmds
        }
    }

    impl Cmd<()> for MockCmdRunner {
        fn run(&self, cmd: &String) -> Result<(), Box<dyn Error>> {
            dbg!(cmd);
            self.push(cmd.clone());
            Ok(())
        }
    }

    impl Cmd<String> for MockCmdRunner {
        fn run(&self, cmd: &String) -> Result<String, Box<dyn Error>> {
            dbg!(cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\"" => {
                    Ok("width: 160\nheight: 90".to_string())
                }
                "tmux new-window -Pd -t test -n code -c /tmp -F \"#{window_id}\"" => {
                    Ok("@1".to_string())
                }
                "tmux new-window -Pd -t test -n infrastructure -c /tmp -F \"#{window_id}\"" => {
                    Ok("@2".to_string())
                }
                "tmux split-window -Pd -t test:@1 -h -c /tmp -F \"#{pane_id}\"" => {
                    Ok("%1".to_string())
                }
                "tmux split-window -Pd -t test:@1 -v -c /tmp/src -F \"#{pane_id}\"" => {
                    Ok("%2".to_string())
                }
                "tmux split-window -Pd -t test:@2 -h -c /tmp/one -F \"#{pane_id}\"" => {
                    Ok("%3".to_string())
                }
                "tmux split-window -Pd -t test:@2 -h -c /tmp/two -F \"#{pane_id}\"" => {
                    Ok("%4".to_string())
                }
                "tmux split-window -Pd -t test:@2 -h -c /tmp/three -F \"#{pane_id}\"" => {
                    Ok("%3".to_string())
                }
                _ => Ok("".to_string()),
            }
        }
    }

    impl Cmd<bool> for MockCmdRunner {
        fn run(&self, cmd: &String) -> Result<bool, Box<dyn Error>> {
            dbg!(cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux has-session -t test" => Ok(false),
                _ => Ok(true),
            }
        }
    }

    impl CmdRunner for MockCmdRunner {}
}
