use crate::app::manager::session::SessionManager;
use crate::common::muxer::test::MockMultiplexer;
use std::fs;

use super::session_test_support::*;

#[test]
fn session_start() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_session_start");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create valid.yaml config
    let config_file = test_config_dir.join("valid.yaml");
    fs::write(
        &config_file,
        r#"---
name: valid
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write valid.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Set expectations for the `switch` method - should return false (session doesn't exist yet)
    mock_multiplexer
        .expect_switch()
        .withf(|name, skip_attach| name == "valid" && !*skip_attach)
        .returning(|_, _| Ok(false));

    // Set expectations for the `start` method
    mock_multiplexer
        .expect_start()
        .withf(|session, env_vars, skip_attach, skip_cmds| {
            // Verify session name and env vars contain LAIO_CONFIG and LAIO_VARS
            session.name == "valid"
                && env_vars.iter().any(|(k, _)| *k == "LAIO_CONFIG")
                && env_vars.iter().any(|(k, _)| *k == "LAIO_VARS")
                && !*skip_attach
                && !*skip_cmds
        })
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(&Some("valid".to_string()), &None, &[], false, false, false);
    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_with_default_fallback() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_default_fallback");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create _default.yaml with template variables
    let default_config = test_config_dir.join("_default.yaml");
    fs::write(
        &default_config,
        r#"---
name: {{ session_name }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write _default.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Switch should return false (session doesn't exist yet)
    mock_multiplexer
        .expect_switch()
        .withf(|name, _| name == "nonexistent")
        .returning(|_, _| Ok(false));

    // Start should be called with session_name auto-injected
    mock_multiplexer
        .expect_start()
        .withf(|session, _, _, _| session.name == "nonexistent")
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Start with non-existent config should fallback to _default.yaml
    let res = session_manager.start(
        &Some("nonexistent".to_string()),
        &None,
        &[],
        false,
        false,
        false,
    );

    if let Err(ref e) = res {
        log::error!("Test failed with error: {:?}", e);
    }
    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_generates_default_if_missing() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_generate_default");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    let default_config = test_config_dir.join("_default.yaml");

    // Ensure _default.yaml doesn't exist
    assert!(!default_config.exists());

    let mut mock_multiplexer = MockMultiplexer::new();

    // Switch should return false
    mock_multiplexer.expect_switch().returning(|_, _| Ok(false));

    // Start should be called with auto-generated default
    mock_multiplexer
        .expect_start()
        .withf(|session, _, _, _| {
            // session_name should be auto-injected, overriding the default "changeme"
            session.name == "myproject"
        })
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // Start should auto-generate _default.yaml
    let res = session_manager.start(
        &Some("myproject".to_string()),
        &None,
        &[],
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Verify _default.yaml was created
    assert!(default_config.exists());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_prefers_existing_config_over_default() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_prefer_existing");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create both _default.yaml and specific config
    fs::write(
        test_config_dir.join("_default.yaml"),
        r#"---
name: default_session
path: /default
windows:
  - name: default
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write _default.yaml");

    fs::write(
        test_config_dir.join("myconfig.yaml"),
        r#"---
name: specific_session
path: /specific
windows:
  - name: specific
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write myconfig.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    mock_multiplexer.expect_switch().returning(|_, _| Ok(false));

    // Should use the specific config, NOT the default
    mock_multiplexer
        .expect_start()
        .withf(|session, _, _, _| session.name == "specific_session")
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(
        &Some("myconfig".to_string()),
        &None,
        &[],
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_with_variable_substitution_in_name() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_var_substitution");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create config with template variable in session name
    let config_file = test_config_dir.join("myproject.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ session_name }}-{{ branch }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write myproject.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Switch should be called with the FINAL substituted name
    mock_multiplexer
        .expect_switch()
        .withf(|name, skip_attach| name == "myproject-feature" && !*skip_attach)
        .returning(|_, _| Ok(false));

    // Start should be called with substituted session name
    mock_multiplexer
        .expect_start()
        .withf(|session, _, _, _| session.name == "myproject-feature")
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(
        &Some("myproject".to_string()),
        &None,
        &["branch=feature".to_string()],
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_switches_to_existing_session_with_substituted_name() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_switch_substituted");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create config with template variable in session name
    let config_file = test_config_dir.join("project.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ session_name }}-{{ env }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write project.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Switch should be called with substituted name and return true (session exists)
    mock_multiplexer
        .expect_switch()
        .withf(|name, skip_attach| name == "project-dev" && !*skip_attach)
        .returning(|_, _| Ok(true));

    // Start should NOT be called since session already exists
    mock_multiplexer.expect_start().times(0);

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(
        &Some("project".to_string()),
        &None,
        &["env=dev".to_string()],
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_with_multiple_variables_in_name() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_multi_var_name");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create config with multiple template variables in session name
    let config_file = test_config_dir.join("api.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ session_name }}-{{ env }}-{{ region }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write api.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Switch should be called with fully substituted name
    mock_multiplexer
        .expect_switch()
        .withf(|name, skip_attach| name == "api-prod-us-east" && !*skip_attach)
        .returning(|_, _| Ok(false));

    // Start should be called with fully substituted session name
    mock_multiplexer
        .expect_start()
        .withf(|session, _, _, _| session.name == "api-prod-us-east")
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(
        &Some("api".to_string()),
        &None,
        &["env=prod".to_string(), "region=us-east".to_string()],
        false,
        false,
        false,
    );

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}

#[test]
fn session_start_without_variables_uses_session_name_only() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_no_var_name");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create config that only uses session_name (no additional vars)
    let config_file = test_config_dir.join("simple.yaml");
    fs::write(
        &config_file,
        r#"---
name: {{ session_name }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write simple.yaml");

    let mut mock_multiplexer = MockMultiplexer::new();

    // Switch should be called with just the session_name
    mock_multiplexer
        .expect_switch()
        .withf(|name, skip_attach| name == "simple" && !*skip_attach)
        .returning(|_, _| Ok(false));

    // Start should be called with session_name only
    mock_multiplexer
        .expect_start()
        .withf(|session, _, _, _| session.name == "simple")
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(&Some("simple".to_string()), &None, &[], false, false, false);

    assert!(res.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
}
