use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::common::cmd::{Runner, Type};

#[derive(Debug)]
pub(crate) struct Client<R: Runner> {
    pub cmd_runner: Rc<R>,
    cmds: RefCell<VecDeque<Type>>,
}

impl<R: Runner> Client<R> {
    pub(crate) fn new(cmd_runner: Rc<R>) -> Self {
        Self {
            cmd_runner,
            cmds: RefCell::new(VecDeque::new()),
        }
    }
}
