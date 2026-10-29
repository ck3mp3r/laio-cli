use std::time::{Duration, Instant};

use log::LevelFilter;
use miette::Result;

use super::shell_runner::should_print_prompt;
use crate::cmd_basic;
use crate::cmd_forget;
use crate::cmd_verbose;
use crate::common::cmd::{Cmd, ShellRunner, Type};

const ALL_LEVELS: [LevelFilter; 6] = [
    LevelFilter::Off,
    LevelFilter::Error,
    LevelFilter::Warn,
    LevelFilter::Info,
    LevelFilter::Debug,
    LevelFilter::Trace,
];

#[test]
fn should_print_prompt_is_false_for_basic_at_every_level() {
    for level in ALL_LEVELS {
        assert!(
            !should_print_prompt(&cmd_basic!("echo", args = ["hi"]), level),
            "Basic should stay silent at {level:?}"
        );
    }
}

#[test]
fn should_print_prompt_is_true_for_verbose_at_every_level() {
    for level in ALL_LEVELS {
        assert!(
            should_print_prompt(&cmd_verbose!("echo", args = ["hi"]), level),
            "Verbose should echo at {level:?}"
        );
    }
}

#[test]
fn should_print_prompt_for_forget_follows_the_debug_threshold() {
    for level in [
        LevelFilter::Off,
        LevelFilter::Error,
        LevelFilter::Warn,
        LevelFilter::Info,
    ] {
        assert!(
            !should_print_prompt(&cmd_forget!("echo", args = ["hi"]), level),
            "Forget should stay silent at {level:?}"
        );
    }

    for level in [LevelFilter::Debug, LevelFilter::Trace] {
        assert!(
            should_print_prompt(&cmd_forget!("echo", args = ["hi"]), level),
            "Forget should echo at {level:?}"
        );
    }
}

#[test]
fn forget_inherits_stdout() -> Result<()> {
    let runner = ShellRunner::new();

    let output: String = runner.run(&cmd_forget!("echo", args = ["forget-inherits-stdout"]))?;

    assert_eq!(output, "", "forget should not capture stdout");
    Ok(())
}

#[test]
fn forget_drains_stderr_until_the_child_exits() -> Result<()> {
    let runner = ShellRunner::new();
    let started = Instant::now();

    let _: () = runner.run(&cmd_forget!("sh", args = ["-c", "sleep 1; exit 0"]))?;

    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(900),
        "forget should block on the stderr drain until the child exits, elapsed {elapsed:?}"
    );
    Ok(())
}

#[test]
fn forget_returns_instantly_for_a_wrapped_daemon() -> Result<()> {
    let runner = ShellRunner::new();
    let started = Instant::now();

    let _: () = runner.run(&cmd_forget!(
        "sh",
        args = ["-c", "nohup sleep 5 > /dev/null 2>&1 </dev/null & disown"]
    ))?;

    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "the wrapper should return instantly, elapsed {elapsed:?}"
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
fn basic_captures_stdout() -> Result<()> {
    let runner = ShellRunner::new();
    let output: String = runner.run(&cmd_basic!("echo", args = ["hi"]))?;

    assert_eq!(output, "hi");
    Ok(())
}
