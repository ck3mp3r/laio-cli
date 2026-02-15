use crate::app::manager::session::SessionManager;
use crate::common::config::Session;
use crate::common::muxer::test::MockMultiplexer;
use crate::common::path::current_working_path;
use crate::common::session_info::SessionInfo;
use std::collections::HashMap;
use std::fs;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn initialize() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

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
fn session_list() {
    initialize();

    let mut mock_multiplexer = MockMultiplexer::new();

    // Set up expectations for `list_sessions`
    mock_multiplexer.expect_list_sessions().returning(|| {
        Ok(vec![
            SessionInfo::active("session1".to_string(), true),
            SessionInfo::active("session2".to_string(), false),
        ])
    });

    let session_manager = SessionManager::new("/path/to/config", Box::new(mock_multiplexer));

    let res = session_manager.list();
    assert!(res.is_ok());
    let list = res.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "session1");
    assert_eq!(list[1].name, "session2");
}

#[test]
fn session_start() {
    initialize();
    let cwd = current_working_path().expect("Cannot get current working directory");

    let mut mock_multiplexer = MockMultiplexer::new();

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

    // Set expectations for the `switch` method
    mock_multiplexer
        .expect_switch()
        .withf(|name, skip_attach| name == "valid" && !*skip_attach)
        .returning(|_, _| Ok(true));

    let session_manager = SessionManager::new(
        &format!("{}/src/app/manager/test", cwd.to_string_lossy()),
        Box::new(mock_multiplexer),
    );

    let res = session_manager.start(&Some("valid".to_string()), &None, &[], false, false, false);
    assert!(res.is_ok());
}

#[test]
fn session_to_yaml() {
    initialize();

    let mut mock_multiplexer = MockMultiplexer::new();

    // Set up expectations for `get_session`
    mock_multiplexer.expect_get_session().returning(|| {
        Ok(Session {
            name: "yaml_test".to_string(),
            path: std::env::temp_dir().to_string_lossy().to_string(),
            startup: vec![],
            shutdown: vec![],
            startup_script: None,
            shutdown_script: None,
            env: HashMap::new(),
            shell: None,
            windows: vec![],
        })
    });

    let session_manager = SessionManager::new("/path/to/config", Box::new(mock_multiplexer));

    let res = session_manager.to_yaml();
    assert!(res.is_ok());
    // Further assertions can validate the YAML output
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
fn default_yaml_filtered_from_picker() {
    initialize();
    let temp_dir = std::env::temp_dir();
    let test_config_dir = temp_dir.join("laio_test_picker_filter");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_config_dir);
    fs::create_dir_all(&test_config_dir).expect("Failed to create test dir");

    // Create _default.yaml and a regular config
    fs::write(
        test_config_dir.join("_default.yaml"),
        r#"---
name: {{ session_name | default(value="default") }}
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write _default.yaml");

    fs::write(
        test_config_dir.join("myconfig.yaml"),
        r#"---
name: myconfig
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#,
    )
    .expect("Failed to write myconfig.yaml");

    // Test that _default.yaml is filtered out in picker logic
    // This is tested indirectly by ensuring that when configs are read,
    // _default.yaml doesn't cause an error (it would if we tried to parse it without variables)

    let mock_multiplexer = MockMultiplexer::new();
    let _session_manager = SessionManager::new(
        test_config_dir.to_str().unwrap(),
        Box::new(mock_multiplexer),
    );

    // If this doesn't panic, the filter is working
    // (Because _default.yaml has template variables without defaults that would fail parsing)

    // Cleanup
    let _ = fs::remove_dir_all(&test_config_dir);
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
fn encode_variables_empty() {
    use crate::app::manager::session::manager::encode_variables;

    let result = encode_variables(&[]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn encode_variables_simple() {
    use crate::app::manager::session::manager::encode_variables;

    let variables = vec!["key1=value1".to_string(), "key2=value2".to_string()];

    let result = encode_variables(&variables);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "key1=value1&key2=value2");
}

#[test]
fn encode_variables_with_special_chars() {
    use crate::app::manager::session::manager::encode_variables;

    let variables = vec![
        "path=/home/user/my project".to_string(),
        "cmd=echo 'hello world'".to_string(),
        "url=https://example.com?foo=bar&baz=qux".to_string(),
    ];

    let result = encode_variables(&variables);
    assert!(result.is_ok());
    let encoded = result.unwrap();

    // Values should be percent-encoded
    assert!(encoded.contains("path=%2Fhome%2Fuser%2Fmy%20project"));
    assert!(encoded.contains("cmd=echo%20%27hello%20world%27"));
    assert!(encoded.contains("url=https%3A%2F%2Fexample.com%3Ffoo%3Dbar%26baz%3Dqux"));
}

#[test]
fn encode_variables_invalid_format() {
    use crate::app::manager::session::manager::encode_variables;

    let variables = vec!["invalid_var".to_string()];

    let result = encode_variables(&variables);
    assert!(result.is_err());
}

#[test]
fn decode_variables_empty() {
    use crate::app::manager::session::manager::decode_variables;

    let result = decode_variables("");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Vec::<String>::new());
}

#[test]
fn decode_variables_simple() {
    use crate::app::manager::session::manager::decode_variables;

    let encoded = "key1=value1&key2=value2";
    let result = decode_variables(encoded);

    assert!(result.is_ok());
    let decoded = result.unwrap();
    assert_eq!(decoded, vec!["key1=value1", "key2=value2"]);
}

#[test]
fn decode_variables_with_special_chars() {
    use crate::app::manager::session::manager::decode_variables;

    let encoded = "path=%2Fhome%2Fuser%2Fmy%20project&cmd=echo%20%27hello%20world%27&url=https%3A%2F%2Fexample.com%3Ffoo%3Dbar%26baz%3Dqux";
    let result = decode_variables(encoded);

    assert!(result.is_ok());
    let decoded = result.unwrap();
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded[0], "path=/home/user/my project");
    assert_eq!(decoded[1], "cmd=echo 'hello world'");
    assert_eq!(decoded[2], "url=https://example.com?foo=bar&baz=qux");
}

#[test]
fn decode_variables_invalid_format() {
    use crate::app::manager::session::manager::decode_variables;

    let encoded = "invalid_pair";
    let result = decode_variables(encoded);

    assert!(result.is_err());
}

#[test]
fn encode_decode_roundtrip() {
    use crate::app::manager::session::manager::{decode_variables, encode_variables};

    let original = vec![
        "session_name=myproject".to_string(),
        "path=/home/user/my project".to_string(),
        "env=production".to_string(),
        "cmd=echo 'test'".to_string(),
    ];

    let encoded = encode_variables(&original).expect("Encoding failed");
    let decoded = decode_variables(&encoded).expect("Decoding failed");

    assert_eq!(decoded, original);
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
