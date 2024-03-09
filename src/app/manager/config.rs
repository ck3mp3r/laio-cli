use anyhow::{Error, Result};
use std::{
    env::{self, var},
    fs::{self},
    io::stdin,
    path::PathBuf,
    rc::Rc,
};

use crate::{
    app::{cmd::CmdRunner, cmd::CommandType, config::Session},
    cmd_basic, cmd_forget,
    util::path::{current_working_path, to_absolute_path},
};

pub(crate) const TEMPLATE: &str = include_str!("tmpl.yaml");
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

    pub(crate) fn create(&self, name: &Option<String>, copy: &Option<String>) -> Result<()> {
        let current_path = name
            .as_ref()
            .map(|_| current_working_path())
            .unwrap_or(Ok(".".into()))?;

        let config_file = match name {
            Some(name) => {
                self.cmd_runner
                    .run(&cmd_basic!("mkdir -p {}", self.config_path))?;
                format!("{}/{}.yaml", self.config_path, name)
            }
            None => ".laio.yaml".to_string(),
        };

        match copy {
            Some(copy_name) => {
                let source = format!("{}/{}.yaml", self.config_path, copy_name);
                self.cmd_runner
                    .run(&cmd_forget!("cp {} {}", source, config_file))?;
            }
            None => {
                let template = TEMPLATE
                    .replace("{name}", name.as_deref().unwrap_or("changeme"))
                    .replace("{path}", &current_path.to_string_lossy());
                self.cmd_runner
                    .run(&cmd_forget!("echo '{}' > {}", template, config_file))?;
            }
        }

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| DEFAULT_EDITOR.to_string());
        self.cmd_runner
            .run(&cmd_forget!("{} {}", editor, config_file))
    }

    pub(crate) fn edit(&self, name: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_forget!(
            "{} {}/{}.yaml",
            var("EDITOR").unwrap_or_else(|_| "vim".to_string()),
            self.config_path,
            name
        ))
    }

    pub(crate) fn link(&self, name: &str, file: &str) -> Result<()> {
        let source = to_absolute_path(file)?;
        self.cmd_runner.run(&cmd_forget!(
            "ln -s {} {}/{}.yaml",
            source.to_string_lossy(),
            self.config_path,
            name
        ))
    }

    pub(crate) fn validate(&self, name: &Option<String>, file: &str) -> Result<()> {
        let config = match name {
            Some(name) => format!("{}/{}.yaml", &self.config_path, name),
            None => PathBuf::from(&file)
                .canonicalize()
                .map_err(|_e| Error::msg(format!("Failed to read config: {}.", file)))?
                .to_string_lossy()
                .into_owned(),
        };
        let _ = Session::from_config(&PathBuf::from(&config))?;
        Ok(())
    }

    pub(crate) fn delete(&self, name: &str, force: bool) -> Result<()> {
        if !force {
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

    pub(crate) fn list(&self) -> Result<Vec<String>> {
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

        Ok(entries)
    }

    #[cfg(test)]
    pub(crate) fn cmd_runner(&self) -> &R {
        &self.cmd_runner
    }
}
