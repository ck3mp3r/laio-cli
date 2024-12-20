use std::rc::Rc;

use crate::common::{
    cmd::{Runner, ShellRunner},
    mux::Multiplexer,
};

use super::Client;

pub(crate) struct Tmux<R: Runner = ShellRunner> {
    client: Client<R>,
}

impl Tmux {
    /// Constructor with a default Runner implementation
    pub fn new() -> Self {
        Self::new_with_runner(ShellRunner::new())
    }
}

impl<R: Runner> Tmux<R> {
    /// Constructor with a specific Runner
    pub fn new_with_runner(runner: R) -> Self {
        Self {
            client: Client::new(Rc::new(runner)),
        }
    }
}

impl<R: Runner> Multiplexer for Tmux<R> {}
