use miette::Result;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Type {
    Basic(String),
    Verbose(String),
    Forget(String),
}

pub(crate) trait Runner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &Type) -> Result<T>;
}

#[macro_export]
macro_rules! cmd_forget {
    ($($arg:tt)*) => {
        Type::Forget(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_basic {
    ($($arg:tt)*) => {
        Type::Basic(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_verbose {
    ($($arg:tt)*) => {
        Type::Verbose(format!($($arg)*))
    };
}
