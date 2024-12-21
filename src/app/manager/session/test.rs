use crate::common::mux::test::MockMultiplexer;

#[test]
fn test_mock_multiplexer() {
    use crate::common::config::Session;

    let mut mock_multiplexer = MockMultiplexer::new();

    // Set expectation for `list_sessions`
    mock_multiplexer
        .expect_list_sessions()
        .returning(|| Ok(vec!["session1".to_string(), "session2".to_string()]));

    // Set expectation for `start`
    mock_multiplexer
        .expect_start()
        .withf(|session, config, skip_attach, skip_cmds| {
            session.name == "test_session" && config == "some_config" && *skip_attach && !*skip_cmds
        })
        .returning(|_, _, _, _| Ok(()));

    // Example usage in a test
    let sessions = mock_multiplexer.list_sessions().unwrap();
    assert_eq!(sessions, vec!["session1", "session2"]);

    let session = Session {
        name: "test_session".to_string(),
        ..Default::default() // Assuming `Session` implements `Default`
    };
    mock_multiplexer
        .start(&session, "some_config", true, false)
        .unwrap();
}
