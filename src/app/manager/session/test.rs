use crate::app::manager::session::SessionManager;
use crate::common::config::Session;
use crate::common::muxer::test::MockMultiplexer;
use crate::common::path::current_working_path;
use crate::common::session_info::SessionInfo;
use std::collections::HashMap;
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

    // Set up expectations for `stop`
    mock_multiplexer
        .expect_stop()
        .withf(|name, skip_cmds, stop_all, stop_other| {
            name.as_deref() == Some("foo") && !*skip_cmds && !*stop_all && !*stop_other
        })
        .returning(|_, _, _, _| Ok(()));

    let session_manager = SessionManager::new("/path/to/config", Box::new(mock_multiplexer));

    let res = session_manager.stop(&Some("foo".to_string()), false, false, false);
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
        .withf(|session, config, skip_attach, skip_cmds| {
            session.name == "valid" && config.is_empty() && !*skip_attach && !*skip_cmds
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
