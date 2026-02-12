use crate::{
    app::ConfigManager,
    common::cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
};

use std::{env::set_var, rc::Rc};

#[test]
fn config_create() {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.to_str().unwrap().trim_end_matches("/");
    set_var("EDITOR", "vim");
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    let expected_editor_cmd = format!("vim {temp_path}/test.yaml");

    cmd_unit
        .expect_run()
        .times(1)
        .withf(move |cmd| {
            matches!(
                cmd,
                Type::Forget(content)
                if format!(
                    "{} {}",
                    content.get_program().to_string_lossy(),
                    content.get_args()
                        .map(|arg| arg.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(" ")
                ) == expected_editor_cmd
            )
        })
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(temp_path, Rc::clone(&cmd_runner));
    cfg.create(&Some("test".to_string()), &None).unwrap();
}

#[test]
fn config_edit() {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.to_str().unwrap();
    set_var("EDITOR", "vim");
    let session_name = "test";
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf({
            let temp_path = temp_path.to_string(); // Clone temp_path for the closure
            move |cmd| {
                matches!(
                    cmd,
                    Type::Forget(content)
                    if format!(
                        "{} {}",
                        content.get_program().to_string_lossy(),
                        content.get_args()
                            .map(|arg| arg.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" ")
                    ) == format!("vim {temp_path}/test.yaml")
                )
            }
        })
        .returning(|_| Ok(()));
    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });
    let cfg = ConfigManager::new(temp_path, Rc::clone(&cmd_runner));
    cfg.edit(session_name).unwrap();
}

#[test]
fn config_validate_no_windows() {
    let session_name = "no_windows";
    let cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.to_str().unwrap();
    let cfg = ConfigManager::new(config_path, Rc::clone(&cmd_runner));

    cfg.validate(&Some(session_name.to_string()), ".laio.yaml")
        .expect_err("Expected missing windows")
        .to_string();
}

#[test]
fn config_validate_multiple_zoom() {
    let session_name = "multi_zoom";
    let cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let config_path = "./src/common/config/test";
    let cfg = ConfigManager::new(config_path, Rc::clone(&cmd_runner));

    cfg.validate(&Some(session_name.to_string()), ".laio.yaml")
        .expect_err("Multiple pane zoom attributes per window detected!")
        .to_string();
}
#[test]
fn config_validate_multiple_focus() {
    let session_name = "multi_focus";
    let cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let config_path = "./src/common/config/test";
    let cfg = ConfigManager::new(config_path, Rc::clone(&cmd_runner));

    cfg.validate(&Some(session_name.to_string()), ".laio.yaml")
        .expect_err("Multiple pane focus attributes per window detected!")
        .to_string();
}

#[test]
fn config_create_uses_default_yaml() {
    use std::fs;

    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join("laio_test_create_with_default");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");

    set_var("EDITOR", "vim");

    // Create custom _default.yaml
    let custom_default = r#"---
name: {{ session_name | default(value="custom") }}
path: {{ path }}
# This is a custom default template
windows:
  - name: custom-window
    panes:
      - flex: 1
"#;

    fs::write(test_dir.join("_default.yaml"), custom_default)
        .expect("Failed to write custom _default.yaml");

    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    // Expect editor to be called
    cmd_unit.expect_run().times(1).returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(test_dir.to_str().unwrap(), Rc::clone(&cmd_runner));
    cfg.create(&Some("myproject".to_string()), &None).unwrap();

    // Verify the created config uses the custom template
    let created_config =
        fs::read_to_string(test_dir.join("myproject.yaml")).expect("Failed to read created config");

    assert!(created_config.contains("# This is a custom default template"));
    assert!(created_config.contains("name: myproject"));
    assert!(created_config.contains("custom-window"));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn config_create_generates_default_if_missing() {
    use std::fs;

    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join("laio_test_create_no_default");

    // Clean up and create fresh test directory
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");

    set_var("EDITOR", "vim");

    // Ensure _default.yaml doesn't exist
    let default_path = test_dir.join("_default.yaml");
    assert!(!default_path.exists());

    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    // Expect editor to be called
    cmd_unit.expect_run().times(1).returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(test_dir.to_str().unwrap(), Rc::clone(&cmd_runner));
    cfg.create(&Some("newproject".to_string()), &None).unwrap();

    // Verify _default.yaml was auto-generated
    assert!(default_path.exists());

    // Verify the created config exists
    let created_config_path = test_dir.join("newproject.yaml");
    assert!(created_config_path.exists());

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}
