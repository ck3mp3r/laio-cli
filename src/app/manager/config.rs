use anyhow::Result;
use std::{
    env::{self, var},
    fs::{self},
    io::stdin,
    rc::Rc,
};

use crate::app::cmd::CmdRunner;

const TEMPLATE: &str = include_str!("tmpl.yaml");
const DEFAULT_EDITOR: &str = "vim";

#[derive(Debug)]
pub(crate) struct ConfigManager<R: CmdRunner> {
    pub config_path: String,
    cmd_runner: Rc<R>,
}

impl<R: CmdRunner> ConfigManager<R> {
    pub(crate) fn new(config_path: &str, cmd_runner: Rc<R>) -> Self {
        Self {
            config_path: config_path.replace("~", env::var("HOME").unwrap().as_str()),
            cmd_runner,
        }
    }

    pub(crate) fn create(&self, name: &str, copy: &Option<String>, pwd: bool) -> Result<()> {
        let config_file = if pwd {
            ".laio.yaml".to_string()
        } else {
            self.cmd_runner
                .run(&format!("mkdir -p {}", self.config_path))?;
            format!("{}/{}.yaml", self.config_path, name)
        };

        match copy {
            Some(copy_name) => {
                let source = format!("{}/{}.yaml", self.config_path, copy_name);
                self.cmd_runner
                    .run(&format!("cp {} {}", source, config_file))?;
            }
            None => {
                let template = TEMPLATE.replace("{name}", name);
                self.cmd_runner
                    .run(&format!("echo '{}' > {}", template, config_file))?;
            }
        }

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| DEFAULT_EDITOR.to_string());
        self.cmd_runner.run(&format!("{} {}", editor, config_file))
    }

    pub(crate) fn edit(&self, name: &str) -> Result<()> {
        self.cmd_runner.run(&format!(
            "{} {}/{}.yaml",
            var("EDITOR").unwrap_or_else(|_| "vim".to_string()),
            self.config_path,
            name
        ))
    }

    pub(crate) fn delete(&self, name: &str, force: &bool) -> Result<()> {
        if !*force {
            println!("Are you sure you want to delete {}? [y/N]", name);
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            if input.trim() != "y" {
                println!("Aborting.");
                return Ok(());
            }
        }
        fs::remove_file(format!("{}/{}.yaml", &self.config_path, name))?;
        Ok(())
    }

    pub(crate) fn list(&self) -> Result<()> {
        let entries = fs::read_dir(&self.config_path)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .map(String::from)
            })
            .collect::<Vec<String>>();

        if entries.is_empty() {
            println!("No configurations found.");
        } else {
            println!("Available Configurations:");
            println!("-------------------------");
            println!("{}", entries.join("\n"));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn cmd_runner(&self) -> &R {
        &self.cmd_runner
    }
}
#[cfg(test)]
mod test {
    use crate::app::cmd::test::MockCmdRunner;

    use super::ConfigManager;
    use super::TEMPLATE;
    use std::{env::var, rc::Rc};

    #[test]
    fn config_new_copy() {
        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let cfg = ConfigManager::new(&"/tmp/laio".to_string(), Rc::clone(&cmd_runner));

        cfg.create(
            &session_name.to_string(),
            &Some(String::from("bla")),
            false,
        )
        .unwrap();
        let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        let cmds = cfg.cmd_runner().cmds().borrow();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0], format!("mkdir -p {}", cfg.config_path));
        assert_eq!(
            cmds[1],
            format!(
                "cp {}/{}.yaml {}/{}.yaml",
                cfg.config_path, "bla", cfg.config_path, session_name
            )
        );
        assert_eq!(cmds[2], format!("{} /tmp/laio/test.yaml", editor));
    }

    #[test]
    fn config_new_local() {
        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let cfg = ConfigManager::new(&".".to_string(), Rc::clone(&cmd_runner));

        cfg.create(&session_name.to_string(), &None, true).unwrap();
        let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        let cmds = cfg.cmd_runner().cmds().borrow();
        let tpl = TEMPLATE.replace("{name}", &session_name.to_string());
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], format!("echo '{}' > .laio.yaml", tpl));
        assert_eq!(cmds[1], format!("{} .laio.yaml", editor));
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
        assert_eq!(cmds[0], format!("{} /tmp/laio/test.yaml", editor));
    }
}
