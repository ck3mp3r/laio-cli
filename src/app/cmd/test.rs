use anyhow::bail;
use log::trace;
use std::cell::RefCell;

use crate::app::cmd::{Cmd, Runner, Type};

impl Type {
    pub fn as_str(&self) -> &str {
        match self {
            Type::Basic(cmd) | Type::Verbose(cmd) => cmd.as_str(),
            Type::Forget(cmd) => cmd.as_str(),
        }
    }
}

// Mock implementation for testing purposes
#[derive(Clone, Debug)]
pub(crate) struct MockRunner {
    cmds: RefCell<Vec<Type>>,
    window_number_generator: RefCell<i32>,
    pane_number_generator: RefCell<i32>,
}

impl MockRunner {
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

    pub fn push(&self, cmd: Type) {
        self.cmds.borrow_mut().push(cmd);
    }

    pub fn cmds(&self) -> &RefCell<Vec<Type>> {
        &self.cmds
    }
}

impl Cmd<()> for MockRunner {
    fn run(&self, cmd: &Type) -> Result<(), anyhow::Error> {
        log::trace!("{:?}", cmd);
        self.push(cmd.clone());
        Ok(())
    }
}

impl Cmd<String> for MockRunner {
    fn run(&self, cmd: &Type) -> Result<String, anyhow::Error> {
        log::trace!("{}", cmd);
        self.push(cmd.clone());
        match cmd.as_str() {
                "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\"" => {
                    Ok("width: 160\nheight: 90".to_string())
                }
                "tmux display-message -p \"#{session_path}\"" => {
                    Ok("/tmp".to_string())
                }
                "tmux display-message -t \"valid\" -p \"#I\"" => {
                    Ok(format!("@{}", self.next_window_id()))
                }
                "tmux new-window -Pd -t \"valid\" -n \"code\" -c \"/tmp\" -F \"#{window_id}\"" => {
                    Ok(format!("@{}", self.next_window_id()))
                }
                "tmux new-window -Pd -t \"valid\" -n \"infrastructure\" -c \"/tmp/one\" -F \"#{window_id}\"" => {
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
                "tmux list-windows -F \"#{window_name} #{window_layout}\"" => Ok(
                    "code 5f31,312x73,0,0,12\nmisc 56be,312x73,0,0{156x73,0,0[156x23,0,0{52x23,0,0,13,51x23,53,0,14,51x23,105,0,15},156x49,0,24,16],155x73,157,0[155x37,157,0,17,155x17,157,38,18,155x17,157,56,19]}".to_string(),
                ),
                "printenv TMUX" => Ok("foo".to_string()),
                "tmux show-options -g base-index" => Ok("base-index 1".to_string()),
                "tmux ls -F \"#{session_name}\"" => Ok(format!("{}","foo\nbar")),
                "tmux show-environment -t \"valid\" LAIO_CONFIG" => Ok("LAIO_CONFIG=./src/app/manager/test/valid.yaml".to_string()),
                "tmux show-environment -t \"foo\" LAIO_CONFIG" => Ok("LAIO_CONFIG=./src/app/manager/test/valid.yaml".to_string()),
                "tmux show-environment -t \"bar\" LAIO_CONFIG" => bail!("Value doesn't exist".to_string()),
                "tmux display-message -p \"#S\"" => Ok("valid".to_string()),
                "tmux list-panes -s -F \"#{pane_id} #{pane_current_path}\"" => Ok(
                    "%12 /tmp\n%13 /tmp/one\n%14 /tmp/two\n%15 /tmp/three\n%16 /tmp\n%17 /tmp/four\n%18 /tmp/five\n%19 /tmp/six".to_string()
                ),
                "tmux list-panes -s -F \"#{pane_id} #{pane_pid}\"" =>{
                    Ok("%12 123\n%13 124".to_string())
                },
                "pgrep -P 123" =>{
                    Ok("1234".to_string())
                },
                "ps -p 1234 -o args=" =>{
                    Ok("$EDITOR".to_string())
                },
                "pgrep -P 124" =>{
                    Ok("1245".to_string())
                },
                "ps -p 1245 -o args=" =>{
                    Ok("foo".to_string())
                },
                _ => {
                    println!("cmd {}", cmd);
                    Ok("".to_string())
                },
            }
    }
}

impl Cmd<bool> for MockRunner {
    fn run(&self, cmd: &Type) -> Result<bool, anyhow::Error> {
        trace!("{}", cmd);
        self.push(cmd.clone());
        match cmd.as_str() {
            "tmux has-session -t \"valid\"" => Ok(false),
            _ => Ok(true),
        }
    }
}

impl Runner for MockRunner {}
