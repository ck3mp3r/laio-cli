use super::session::Session;
use std::path::PathBuf;

#[test]
fn test_from_config_with_variables() {
    let config_path = PathBuf::from("src/common/config/test/templated.yaml");
    let variables = vec![
        "name=my-project".to_string(),
        "path=/home/user/projects".to_string(),
        "window_name=editor".to_string(),
    ];

    let session = Session::from_config(&config_path, Some(&variables)).unwrap();

    assert_eq!(session.name, "my-project");
    assert_eq!(session.path, "/home/user/projects");
    assert_eq!(session.windows.len(), 1);
    assert_eq!(session.windows[0].name, "editor");
}

#[test]
fn test_from_config_with_defaults() {
    let config_path = PathBuf::from("src/common/config/test/templated.yaml");

    let session = Session::from_config(&config_path, None).unwrap();

    // Should use default values from template
    assert_eq!(session.name, "test-session");
    assert_eq!(session.path, "/tmp");
    assert_eq!(session.windows[0].name, "main");
}

#[test]
fn test_from_config_partial_variables() {
    let config_path = PathBuf::from("src/common/config/test/templated.yaml");
    let variables = vec!["name=partial-test".to_string()];

    let session = Session::from_config(&config_path, Some(&variables)).unwrap();

    assert_eq!(session.name, "partial-test");
    assert_eq!(session.path, "/tmp"); // Uses default
    assert_eq!(session.windows[0].name, "main"); // Uses default
}

#[test]
fn test_from_config_with_array_variables() {
    let config_path = PathBuf::from("src/common/config/test/multi-projects.yaml");
    let variables = vec![
        "name=multi-env".to_string(),
        "path=/home/dev".to_string(),
        "projects=web".to_string(),
        "projects=api".to_string(),
        "projects=cli".to_string(),
    ];

    let session = Session::from_config(&config_path, Some(&variables)).unwrap();

    assert_eq!(session.name, "multi-env");
    assert_eq!(session.path, "/home/dev");
    assert_eq!(session.windows.len(), 3);
    assert_eq!(session.windows[0].name, "web");
    assert_eq!(session.windows[1].name, "api");
    assert_eq!(session.windows[2].name, "cli");

    // Check paths are correct
    assert_eq!(session.windows[0].panes[0].path, "/home/dev/web");
    assert_eq!(session.windows[1].panes[0].path, "/home/dev/api");
    assert_eq!(session.windows[2].panes[0].path, "/home/dev/cli");
}
