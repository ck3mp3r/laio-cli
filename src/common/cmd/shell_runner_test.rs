use std::time::{Duration, Instant};

use miette::Result;

use crate::cmd_basic;
use crate::cmd_forget;
use crate::cmd_interactive;
use crate::common::cmd::{Cmd, ShellRunner, Type};

#[test]
fn forget_returns_before_the_child_exits() -> Result<()> {
    let runner = ShellRunner::new();
    let started = Instant::now();

    let _: () = runner.run(&cmd_forget!("sleep", args = ["5"]))?;

    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "forget should not wait for the child, elapsed {elapsed:?}"
    );
    Ok(())
}

#[test]
fn forget_ignores_a_non_zero_exit() -> Result<()> {
    let runner = ShellRunner::new();

    let _: () = runner.run(&cmd_forget!("sh", args = ["-c", "exit 3"]))?;

    Ok(())
}

#[test]
fn interactive_returns_ok_when_the_child_exits_zero() -> Result<()> {
    let runner = ShellRunner::new();

    let _: () = runner.run(&cmd_interactive!("sh", args = ["-c", "exit 0"]))?;

    Ok(())
}

#[test]
fn interactive_returns_err_when_the_child_exits_non_zero() -> Result<()> {
    let runner = ShellRunner::new();

    let result: Result<()> = runner.run(&cmd_interactive!("sh", args = ["-c", "exit 3"]));

    assert!(result.is_err(), "interactive should report a non-zero exit");
    Ok(())
}

#[test]
fn interactive_does_not_capture_stdout() -> Result<()> {
    let runner = ShellRunner::new();

    let output: String = runner.run(&cmd_interactive!("sh", args = ["-c", "echo hi"]))?;

    assert_eq!(output, "");
    Ok(())
}

#[test]
fn basic_captures_stdout() -> Result<()> {
    let runner = ShellRunner::new();
    let output: String = runner.run(&cmd_basic!("echo", args = ["hi"]))?;

    assert_eq!(output, "hi");
    Ok(())
}
