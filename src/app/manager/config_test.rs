use crate::app::{
    cmd_test::test::MockCmdRunner,
    manager::config::{ConfigManager, TEMPLATE},
};

use std::{env::var, rc::Rc};

#[test]
fn config_new_copy() {
    let session_name = "test";
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let cfg = ConfigManager::new(&"/tmp/laio".to_string(), Rc::clone(&cmd_runner));

    cfg.create(&Some(session_name.to_string()), &Some(String::from("bla")))
        .unwrap();
    let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let cmds = cfg.cmd_runner().cmds().borrow();
    assert_eq!(cmds.len(), 3);
    assert_eq!(cmds[0].as_str(), format!("mkdir -p {}", cfg.config_path));
    assert_eq!(
        cmds[1].as_str(),
        format!(
            "cp {}/{}.yaml {}/{}.yaml",
            cfg.config_path, "bla", cfg.config_path, session_name
        )
    );
    assert_eq!(cmds[2].as_str(), format!("{} /tmp/laio/test.yaml", editor));
}

#[test]
fn config_new_local() {
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let cfg = ConfigManager::new(&".".to_string(), Rc::clone(&cmd_runner));

    cfg.create(&None, &None).unwrap();
    let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let cmds = cfg.cmd_runner().cmds().borrow();
    println!("{:?}", cmds);
    let tpl = TEMPLATE
        .replace("{name}", &"changeme")
        .replace("{path}", &".");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].as_str(), format!("echo '{}' > .laio.yaml", tpl));
    assert_eq!(cmds[1].as_str(), format!("{} .laio.yaml", editor));
}

#[test]
fn config_edit() {
    let session_name = "test";
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let cfg = ConfigManager::new(&"/tmp/laio".to_string(), Rc::clone(&cmd_runner));

    cfg.edit(&session_name.to_string()).unwrap();
    let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let cmds = cfg.cmd_runner().cmds().borrow();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].as_str(), format!("{} /tmp/laio/test.yaml", editor));
}

#[test]
fn config_validate_no_windows() {
    let session_name = "no_windows";
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let config_path = &"./src/app/manager/test".to_string();
    let cfg = ConfigManager::new(config_path, Rc::clone(&cmd_runner));

    cfg.validate(&Some(session_name.to_string()), ".laio.yaml")
        .expect_err("Expected missing windows")
        .to_string();
}

#[test]
fn config_validate_multiple_zoom() {
    let session_name = "multi_zoom";
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let config_path = &"./src/app/manager/test".to_string();
    let cfg = ConfigManager::new(config_path, Rc::clone(&cmd_runner));

    cfg.validate(&Some(session_name.to_string()), ".laio.yaml")
        .expect_err("Multiple pane zoom attributes per window detected!")
        .to_string();
}
