use crate::common::{cmd::Type, config::Session};
use miette::{Context, Error, IntoDiagnostic, Result};
use std::{
    env::{self, var},
    fs::{self},
    io::stdin,
    path::PathBuf,
    rc::Rc,
};

use crate::{
    cmd_forget,
    common::{
        cmd::Runner,
        path::{current_working_path, to_absolute_path},
    },
};

pub(crate) const TEMPLATE: &str = include_str!("tmpl.yaml");
const DEFAULT_EDITOR: &str = "vim";

#[derive(Debug)]
pub(crate) struct ConfigManager<R: Runner> {
    pub config_path: String,
    cmd_runner: Rc<R>,
}

impl<R: Runner> ConfigManager<R> {
    pub(crate) fn new(config_path: &str, cmd_runner: Rc<R>) -> Self {
        Self {
            config_path: config_path.replace('~', env::var("HOME").unwrap().as_str()),
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
                format!("{}/{}.yaml", self.config_path, name)
            }
            None => ".laio.yaml".to_string(),
        };

        match copy {
            Some(copy_name) => {
                let source = format!("{}/{}.yaml", self.config_path, copy_name);
                let _: () = self
                    .cmd_runner
                    .run(&cmd_forget!("cp {} {}", source, config_file))
                    .wrap_err(format!("Could not copy '{}' to '{}'", source, config_file))?;
            }
            None => {
                let template = TEMPLATE
                    .replace("{ name }", name.as_deref().unwrap_or("changeme"))
                    .replace("{ path }", &current_path.to_string_lossy());
                let _: () = self
                    .cmd_runner
                    .run(&cmd_forget!("echo '{}' > {}", template, config_file))
                    .wrap_err(format!("Could not create '{}'", config_file))?;
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
        let source = to_absolute_path(file)
            .wrap_err(format!("Failed to get absolute path for '{}'", file))?;
        let source_file = source.to_string_lossy();
        let destination = format!("{}/{}.yaml", self.config_path, name);
        self.cmd_runner
            .run(&cmd_forget!(
                "ln -s \"{}\" \"{}\"",
                &source_file,
                destination
            ))
            .wrap_err(format!(
                "Failed to link '{}' to '{}'",
                &source_file, destination
            ))
    }

    ///TODO: refactor this, code smell
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
            stdin().read_line(&mut input).into_diagnostic()?;
            if input.trim() != "y" {
                println!("Aborting.");
                return Ok(());
            }
        }
        let file = format!("{}/{}.yaml", &self.config_path, name);
        fs::remove_file(&file)
            .into_diagnostic()
            .wrap_err(format!("Failed to delete '{}'", &file))?;
        Ok(())
    }

    pub(crate) fn list(&self) -> Result<Vec<String>> {
        let mut entries = fs::read_dir(&self.config_path)
            .into_diagnostic()
            .wrap_err(format!(
                "Failed to list config entries in '{}'",
                &self.config_path
            ))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .map(String::from)
            })
            .collect::<Vec<String>>();

        entries.sort();
        Ok(entries)
    }
}
