use super::command_line::Cli;
use clap::Parser;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn parse(args: &[&str]) -> Cli {
    Cli::parse_from(std::iter::once("laio").chain(args.iter().copied()))
}

#[test]
fn start_socket_flag_is_parsed() {
    let cli = parse(&["start", "--tmux-socket", "/tmp/test.sock", "--skip-attach"]);
    assert_eq!(cli.tmux_socket.as_deref(), Some("/tmp/test.sock"));
}

#[test]
fn start_socket_flag_absent_is_none() {
    let cli = parse(&["start", "--skip-attach"]);
    assert!(cli.tmux_socket.is_none());
}

#[test]
fn start_socket_resolved_from_env_var() {
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test is serialized via ENV_LOCK; no concurrent env access.
    unsafe { std::env::set_var("LAIO_TMUX_SOCKET", "/tmp/env-test.sock") };
    let cli = parse(&["start", "--skip-attach"]);
    assert!(cli.tmux_socket.is_none());
    let resolved = cli.resolved_socket();
    // SAFETY: restoring env state.
    unsafe { std::env::remove_var("LAIO_TMUX_SOCKET") };
    assert_eq!(resolved.as_deref(), Some("/tmp/env-test.sock"));
}

#[test]
fn start_socket_flag_overrides_env_var() {
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test is serialized via ENV_LOCK; no concurrent env access.
    unsafe { std::env::set_var("LAIO_TMUX_SOCKET", "/tmp/env-value.sock") };
    let cli = parse(&[
        "start",
        "--tmux-socket",
        "/tmp/flag-value.sock",
        "--skip-attach",
    ]);
    let resolved = cli.resolved_socket();
    // SAFETY: restoring env state.
    unsafe { std::env::remove_var("LAIO_TMUX_SOCKET") };
    assert_eq!(resolved.as_deref(), Some("/tmp/flag-value.sock"));
}

#[test]
fn socket_flag_is_global() {
    let cli = parse(&["session", "list", "--tmux-socket", "/tmp/test.sock"]);
    assert_eq!(cli.tmux_socket.as_deref(), Some("/tmp/test.sock"));
}
