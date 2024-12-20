use anyhow::Result;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Basic(String),
    Verbose(String),
    Forget(String),
}

pub(crate) trait Runner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &Type) -> Result<T>;
}
