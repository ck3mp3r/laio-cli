use std::{env, fs};

use miette::{IntoDiagnostic, Result, miette};
use serde_json::{Value, json};

use super::client::{HerdrConnection, config_dir_from, resolve_socket_path};

use super::support_env::*;
use super::support_responses::*;
use super::support_socket::*;

#[test]
fn connect_without_listener_returns_err_with_socket_path() -> Result<()> {
    let path = temp_socket_path("missing");
    let _ = fs::remove_file(&path);

    let error = HerdrConnection::connect_to(&path)
        .err()
        .ok_or_else(|| miette!("missing socket should return Err"))?;

    let report = error.to_string();
    assert!(
        report.contains(path.to_string_lossy().as_ref()),
        "report should contain the socket path: {report}"
    );
    Ok(())
}

#[test]
fn socket_path_uses_xdg_config_home_and_ignores_herdr_socket_path() -> Result<()> {
    let _guard = env_lock()?;
    let xdg_config_home = env::temp_dir().join("laio-herdr-xdg-config");
    let decoy = env::temp_dir().join("laio-herdr-decoy.sock");
    let previous_xdg = env::var_os("XDG_CONFIG_HOME");
    let previous_socket = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
        env::set_var("HERDR_SOCKET_PATH", &decoy);
    }
    let resolved = resolve_socket_path("work");
    restore_env("XDG_CONFIG_HOME", previous_xdg);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    let resolved = resolved?;
    let expected = xdg_config_home
        .join("herdr")
        .join("sessions")
        .join("work")
        .join("herdr.sock");

    assert_eq!(resolved, expected);
    assert_ne!(resolved, decoy);
    Ok(())
}

#[test]
fn config_dir_falls_back_to_home_config() -> Result<()> {
    let home = env::temp_dir().join("laio-herdr-home");
    let config_dir = config_dir_from(None, Some(home.to_string_lossy().to_string()))?;

    assert_eq!(config_dir, home.join(".config").join("herdr"));
    Ok(())
}

#[test]
fn config_dir_requires_home_when_xdg_config_home_is_unset() -> Result<()> {
    let error = config_dir_from(None, None)
        .err()
        .ok_or_else(|| miette!("missing HOME should return Err"))?;

    assert!(error.to_string().contains("home directory"));
    Ok(())
}

#[test]
fn connect_resolves_named_session_socket_and_pings() -> Result<()> {
    let _guard = env_lock()?;
    let xdg_config_home = env::temp_dir().join(format!("lh{}", std::process::id()));
    let previous_xdg = env::var_os("XDG_CONFIG_HOME");

    unsafe {
        env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
    }
    let pong = (|| -> Result<Value> {
        let socket_path = resolve_socket_path("w")?;
        let parent = socket_path
            .parent()
            .ok_or_else(|| miette!("socket path should have a parent directory"))?;
        fs::create_dir_all(parent).into_diagnostic()?;

        let listener = bind_listener(&socket_path)?;
        let handle = spawn_responder(listener, pong_response());
        let mut connection = HerdrConnection::connect("w")?;
        let pong = connection.request("ping", json!({}))?;
        let _ = join_thread(handle)?;
        Ok(pong)
    })();
    restore_env("XDG_CONFIG_HOME", previous_xdg);

    let pong = pong?;
    let _ = fs::remove_dir_all(&xdg_config_home);

    assert_eq!(pong["version"], json!("0.9.5"));
    assert_eq!(pong["protocol"], json!(22));
    Ok(())
}
