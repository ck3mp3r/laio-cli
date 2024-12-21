use crate::{
    app::{manager::config::manager::TEMPLATE, ConfigManager},
    common::cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
};

use std::{env::var, rc::Rc};

#[test]
fn config_new_copy() {
    let session_name = "test";
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "cp"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "vim"))
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(&"/tmp/laio".to_string(), Rc::clone(&cmd_runner));
    cfg.create(&Some(session_name.to_string()), &Some(String::from("bla")))
        .unwrap();

    let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    // Assertions handled by mock expectations
}

#[test]
fn config_new_local() {
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content.starts_with("echo")))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content.starts_with("vim")))
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(&".".to_string(), Rc::clone(&cmd_runner));
    cfg.create(&None, &None).unwrap();

    let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let tpl = TEMPLATE
        .replace("{name}", &"changeme")
        .replace("{path}", &".");
    // Assertions handled by mock expectations
}

#[test]
fn config_edit() {
    let session_name = "test";
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content.starts_with("vim")))
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(&"/tmp/laio".to_string(), Rc::clone(&cmd_runner));
    cfg.edit(&session_name.to_string()).unwrap();

    let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    // Assertions handled by mock expectations
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

    let config_path = &"./src/app/manager/test".to_string();
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

    let config_path = &"./src/app/manager/test".to_string();
    let cfg = ConfigManager::new(config_path, Rc::clone(&cmd_runner));

    cfg.validate(&Some(session_name.to_string()), ".laio.yaml")
        .expect_err("Multiple pane zoom attributes per window detected!")
        .to_string();
}

