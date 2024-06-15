#[cfg(test)]
pub mod test {
    use anyhow::bail;
    use log::trace;
    use std::cell::RefCell;

    use crate::app::cmd::{Cmd, CmdRunner, CommandType};

    impl CommandType {
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
        window_number_generator: RefCell<i32>,
        pane_number_generator: RefCell<i32>,
    }

    impl MockCmdRunner {
        pub fn new() -> Self {
            Self {
                cmds: RefCell::new(vec![]),
                window_number_generator: RefCell::new(0),
                pane_number_generator: RefCell::new(0),
            }
        }

        pub fn next_window_id(&self) -> i32 {
            let mut num = self.window_number_generator.borrow_mut();
            *num += 1;
            *num
        }

        pub fn next_pane_id(&self) -> i32 {
            let mut num = self.pane_number_generator.borrow_mut();
            *num += 1;
            *num
        }

        pub fn push(&self, cmd: CommandType) {
            self.cmds.borrow_mut().push(cmd);
        }

        pub fn cmds(&self) -> &RefCell<Vec<CommandType>> {
            &self.cmds
        }
    }

    impl Cmd<()> for MockCmdRunner {
        fn run(&self, cmd: &CommandType) -> Result<(), anyhow::Error> {
            log::trace!("{:?}", cmd);
            self.push(cmd.clone());
            Ok(())
        }
    }

    impl Cmd<String> for MockCmdRunner {
        fn run(&self, cmd: &CommandType) -> Result<String, anyhow::Error> {
            log::trace!("{}", cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\"" => {
                    Ok("width: 160\nheight: 90".to_string())
                }
                "tmux display-message -p \"#{session_path}\"" => {
                    Ok("/tmp".to_string())
                }
                "tmux new-window -Pd -t \"valid\" -n \"code\" -c \"/tmp\" -F \"#{window_id}\"" => {
                    Ok(format!("@{}", self.next_window_id()))
                }
                "tmux new-window -Pd -t \"valid\" -n \"infrastructure\" -c \"/tmp\" -F \"#{window_id}\"" => {
                    Ok(format!("@{}", self.next_window_id()))
                }
                "tmux split-window -t \"valid\":@1 -c \"/tmp\" -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", self.next_pane_id()))
                }
                "tmux split-window -t \"valid\":@1 -c \"/tmp/src\" -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", self.next_pane_id()))
                }
                "tmux split-window -t \"valid\":@2 -c \"/tmp/one\" -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", self.next_pane_id()))
                }
                "tmux split-window -t \"valid\":@2 -c \"/tmp/two\" -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", self.next_pane_id()))
                }
                "tmux split-window -t \"valid\":@2 -c \"/tmp/three\" -P -F \"#{pane_id}\"" => {
                    Ok(format!("%{}", self.next_pane_id()))
                }
                "tmux display-message -t \"valid\":@1 -p \"#P\"" => Ok(format!("%{}", self.next_pane_id())),
                "tmux display-message -t \"valid\":@2 -p \"#P\"" => Ok(format!("%{}", self.next_pane_id())),
                "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\"" => Ok(
                    "code ce5e,274x86,0,0,1\nmisc 6b9f,274x86,0,0{137x86,0,0[137x27,0,0{42x27,0,0,2,46x27,43,0,6,47x27,90,0,8},137x58,0,28,4],136x86,138,0[136x43,138,0,5,136x21,138,44,10,136x20,138,66{86x20,138,66,11,49x20,225,66,12}]}".to_string(),
                ),
                "printenv TMUX" => Ok("foo".to_string()),
                "tmux show-options -g base-index" => Ok("base-index 1".to_string()),
                "tmux ls -F \"#{session_name}\"" => Ok(format!("{}","foo\nbar")),
                "tmux show-environment -t \"valid\": LAIO_CONFIG" => Ok("LAIO_CONFIG=./src/app/manager/test/valid.yaml".to_string()),
                "tmux show-environment -t \"foo\": LAIO_CONFIG" => Ok("LAIO_CONFIG=./src/app/manager/test/valid.yaml".to_string()),
                "tmux show-environment -t \"bar\": LAIO_CONFIG" => bail!("Value doesn't exist".to_string()),
                _ => {
                    println!("cmd {}", cmd);
                    Ok("".to_string())
                },
            }
        }
    }

    impl Cmd<bool> for MockCmdRunner {
        fn run(&self, cmd: &CommandType) -> Result<bool, anyhow::Error> {
            trace!("{}", cmd);
            self.push(cmd.clone());
            match cmd.as_str() {
                "tmux has-session -t \"valid\"" => Ok(false),
                _ => Ok(true),
            }
        }
    }

    impl CmdRunner for MockCmdRunner {}
}
