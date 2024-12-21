use crate::common::config::{FlexDirection, Window};
use crate::common::mux::multiplexer::Multiplexer;
use crate::common::mux::test::MockMultiplexer;

use std::collections::HashMap;

fn default_path() -> String {
    "/tmp".to_string() // Example implementation of `default_path`
}

#[test]
fn test_mock_multiplexer() {
    use crate::common::config::Session;

    let mut mock_multiplexer = MockMultiplexer::new();

    // Set expectation for `start`
    mock_multiplexer
        .expect_start()
        .withf(|session, config, skip_attach, skip_cmds| {
            session.name == "test_session" && config == "some_config" && *skip_attach && !*skip_cmds
        })
        .returning(|_, _, _, _| Ok(()));

    // Create a sample Window (assuming `Window` has similar requirements)
    let window = Window {
        name: "main".to_string(),
        panes: vec![], // Replace with appropriate fields
        flex_direction: FlexDirection::Row,
    };

    // Initialize `Session`
    let session = Session {
        name: "test_session".to_string(),
        path: default_path(),
        startup: vec!["echo 'Starting up'".to_string()],
        shutdown: vec!["echo 'Shutting down'".to_string()],
        env: HashMap::new(),
        windows: vec![window],
    };

    // Call the mock
    let result = mock_multiplexer.start(&session, "some_config", true, false);
    assert!(result.is_ok());
}
