use crate::app::manager::session::SessionManager;
use crate::common::muxer::test::MockMultiplexer;
use std::fs;

use super::session_test_support::*;

#[test]
fn session_stop() {
    initialize();

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path - return None (session not found or not a laio session)
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("foo"))
        .returning(|_| Ok(None));

    // Set up expectations for `stop`
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("foo")
                && session.is_none()
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new("/path/to/config", Box::new(mock_multiplexer));

    let res = session_manager.stop(&Some("foo".to_string()), &[], false, false, false);
    assert!(res.is_ok());
}

#[test]
fn session_stop_with_variables() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_stop_with_vars");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create a templated config
    let config_file = test_config_dir.join("mytemplate.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ project_name }}-{{ env }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write mytemplate.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path to return the config path
    let config_path_str = config_file.to_str().unwrap().to_string();
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("mytemplate"))
        .returning(move |_| Ok(Some(config_path_str.clone())));

    // Stop should be called with the session object containing shutdown commands
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            // Name should be the config name
            name.as_deref() == Some("mytemplate")
                // Session should be provided with the resolved name
                && session.as_ref().map(|s| s.name.as_str()) == Some("webapp-dev")
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Stop with variables should resolve the session name
    let variables = vec!["project_name=webapp".to_string(), "env=dev".to_string()];
    let res = session_manager.stop(
        &Some("mytemplate".to_string()),
        &variables,
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_stop_with_variables_and_default_fallback() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_stop_default_fallback");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create _default.yaml with template variables
    let default_config = test_config_dir.join("_default.yaml");
    fs::write(
        &default_config,
        r#"---
name: {{ session_name }}-{{ env }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write _default.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path to return the default config path
    let default_config_str = default_config.to_str().unwrap().to_string();
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("myproject"))
        .returning(move |_| Ok(Some(default_config_str.clone())));

    // Stop should be called with the session object
    // session_name should be auto-injected
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("myproject")
                && session.as_ref().map(|s| s.name.as_str()) == Some("myproject-prod")
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Stop non-existent config should fallback to _default.yaml with session_name auto-injected
    let variables = vec!["env=prod".to_string()];
    let res = session_manager.stop(
        &Some("myproject".to_string()),
        &variables,
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_stop_without_variables() {
    initialize();

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path - return None (session not found)
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("simple-session"))
        .returning(|_| Ok(None));

    // Stop should be called with the name as-is (no template resolution)
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("simple-session")
                && session.is_none()
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new("/path/to/config", Box::new(mock_multiplexer));

    // Stop without variables should use name directly
    let res = session_manager.stop(
        &Some("simple-session".to_string()),
        &[],
        false,
        false,
        false,
    );

    assert!(res.is_ok());
}

#[test]
fn session_stop_retrieves_stored_variables() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_stop_retrieve_vars");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create a templated config (same as in session_stop_with_variables test that works)
    let config_file = test_config_dir.join("mytemplate.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ project_name }}-{{ env }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write mytemplate.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path to return the config path
    let config_path_str = config_file.to_str().unwrap().to_string();
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("mytemplate"))
        .returning(move |_| Ok(Some(config_path_str.clone())));

    // Mock get_session_variables to return stored variables (simulating LAIO_VARS)
    mock_multiplexer
        .expect_get_session_variables()
        .with(mockall::predicate::eq("mytemplate"))
        .returning(|_| {
            Ok(Some(vec![
                "session_name=mytemplate".to_string(),
                "path=/tmp".to_string(),
                "project_name=webapp".to_string(),
                "env=dev".to_string(),
            ]))
        });

    // Stop should be called with the session object
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("mytemplate")
                && session.as_ref().map(|s| s.name.as_str()) == Some("webapp-dev")
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Stop WITHOUT providing variables - should retrieve from session
    let res = session_manager.stop(&Some("mytemplate".to_string()), &[], false, false, false);

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_stop_no_stored_variables_uses_defaults() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_stop_no_vars");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create a simple config
    let config_file = test_config_dir.join("simple.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ session_name }}
path: {{ path }}
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write simple.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path to return the config path
    let config_path_str = config_file.to_str().unwrap().to_string();
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("simple"))
        .returning(move |_| Ok(Some(config_path_str.clone())));

    // Mock get_session_variables to return None (backward compatibility)
    mock_multiplexer
        .expect_get_session_variables()
        .with(mockall::predicate::eq("simple"))
        .returning(|_| Ok(None));

    // Stop should be called with defaults (session_name + path)
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("simple")
                && session.as_ref().map(|s| s.name.as_str()) == Some("simple")
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Stop without variables and no stored variables - should use defaults
    let res = session_manager.stop(&Some("simple".to_string()), &[], false, false, false);

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_stop_user_variables_override_stored() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_stop_override_vars");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create a templated config (same as other working tests)
    let config_file = test_config_dir.join("override.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ project_name }}-{{ env }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write override.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Mock get_session_config_path to return the config path
    let config_path_str = config_file.to_str().unwrap().to_string();
    mock_multiplexer
        .expect_get_session_config_path()
        .with(mockall::predicate::eq("override"))
        .returning(move |_| Ok(Some(config_path_str.clone())));

    // get_session_variables should NOT be called when user provides variables
    mock_multiplexer.expect_get_session_variables().times(0);

    // Stop should use user-provided variables, not stored ones
    mock_multiplexer
        .expect_stop()
        .withf(|name, session, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("override")
                && session.as_ref().map(|s| s.name.as_str()) == Some("api-prod")
                && !*skip_cmds
                && !*stop_all
                && !*stop_other
        })
        .returning(|_, _, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Stop WITH user-provided variables - should NOT retrieve from session
    let user_vars = vec!["project_name=api".to_string(), "env=prod".to_string()];
    let res = session_manager.stop(
        &Some("override".to_string()),
        &user_vars,
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}
