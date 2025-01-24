use crate::{
    app::{manager::config::manager::TEMPLATE, ConfigManager},
    common::cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
};

use std::{
    env::{set_var, var},
    rc::Rc,
};

#[test]
fn config_new_copy() {
    let session_name = "test";
    set_var("EDITOR", "vim");
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Forget(content) if content == "cp /tmp/laio/bla.yaml /tmp/laio/test.yaml"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Forget(content) if content == "vim /tmp/laio/test.yaml"))
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new("/tmp/laio", Rc::clone(&cmd_runner));
    cfg.create(&Some(session_name.to_string()), &Some(String::from("bla")))
        .unwrap();

    let _editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
}

#[test]
fn config_new_local() {
    set_var("EDITOR", "vim");
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    // Create the string outside and pass it by value into the closure
    let tpl_replacement = TEMPLATE
        .replace("{ name }", "changeme")
        .replace("{ path }", ".");
    let expected_echo_cmd = format!("echo '{}' > .laio.yaml", tpl_replacement);
    let expected_editor_cmd = "vim .laio.yaml".to_string();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(move |cmd| {
            if let Type::Forget(content) = cmd {
                content == &expected_echo_cmd
            } else {
                false
            }
        })
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(move |cmd| {
            if let Type::Forget(content) = cmd {
                content == &expected_editor_cmd
            } else {
                false
            }
        })
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new(".", Rc::clone(&cmd_runner));
    cfg.create(&None, &None).unwrap();
}

#[test]
fn config_edit() {
    set_var("EDITOR", "vim");
    let session_name = "test";
    let mut cmd_unit = MockCmdUnitMock::new();
    let cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Forget(content) if content == "vim /tmp/laio/test.yaml"))
        .returning(|_| Ok(()));

    let cmd_runner = Rc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    });

    let cfg = ConfigManager::new("/tmp/laio", Rc::clone(&cmd_runner));
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

    let config_path = "./src/common/config/test";
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
