mod model;
mod shell_runner;
pub(crate) use model::Cmd;
pub(crate) use model::Runner;
pub(crate) use model::Type;
pub(crate) use shell_runner::ShellRunner;

#[cfg(test)]
#[path = "../../../test/common/cmd/test.rs"]
pub(crate) mod test;

#[cfg(test)]
#[path = "../../../test/common/cmd/shell_runner_test.rs"]
mod shell_runner_test;
