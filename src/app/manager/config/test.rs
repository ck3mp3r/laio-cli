use crate::{
    app::ConfigManager,
    common::cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
};

use std::{env::set_var, rc::Rc};
use tempfile::tempdir;

#[test]
fn config_new() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();
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
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();
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

    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().to_str().unwrap();
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
