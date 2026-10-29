use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

use clap::ValueEnum;
use miette::{Result, miette};

use super::{Muxer, create_muxer};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> Result<MutexGuard<'static, ()>> {
    ENV_LOCK.lock().map_err(|_| miette!("env lock poisoned"))
}

fn set_laio_muxer(value: Option<&str>) -> Option<OsString> {
    let previous = std::env::var_os("LAIO_MUXER");

    unsafe {
        match value {
            Some(value) => std::env::set_var("LAIO_MUXER", value),
            None => std::env::remove_var("LAIO_MUXER"),
        }
    }

    previous
}

fn restore_laio_muxer(previous: Option<OsString>) {
    unsafe {
        match previous {
            Some(value) => std::env::set_var("LAIO_MUXER", value),
            None => std::env::remove_var("LAIO_MUXER"),
        }
    }
}

#[test]
fn create_muxer_returns_each_explicit_backend() -> Result<()> {
    for muxer in [Muxer::Tmux, Muxer::Zellij, Muxer::Herdr] {
        assert!(create_muxer(&Some(muxer)).is_ok());
    }

    Ok(())
}

#[test]
fn create_muxer_accepts_herdr_from_the_env_var() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_laio_muxer(Some("herdr"));
    let result = create_muxer(&None);
    restore_laio_muxer(previous);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn create_muxer_matches_the_env_var_case_insensitively() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_laio_muxer(Some("HeRdR"));
    let result = create_muxer(&None);
    restore_laio_muxer(previous);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn create_muxer_rejects_unsupported_env_values() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_laio_muxer(Some("screen"));
    let result = create_muxer(&None);
    restore_laio_muxer(previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("unsupported muxer should return Err"))?;

    assert!(
        error.to_string().contains("screen"),
        "report should name the unsupported value: {error}"
    );
    Ok(())
}

#[test]
fn create_muxer_defaults_to_tmux_without_the_env_var() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_laio_muxer(None);
    let result = create_muxer(&None);
    restore_laio_muxer(previous);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn muxer_value_enum_accepts_herdr() -> Result<()> {
    assert!(matches!(Muxer::from_str("herdr", true), Ok(Muxer::Herdr)));
    Ok(())
}
