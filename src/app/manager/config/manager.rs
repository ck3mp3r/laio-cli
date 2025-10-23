use crate::common::{cmd::Type, config::Session};
use miette::{miette, Context, Error, IntoDiagnostic, Result};
use std::{
    env::{self, var},
    fs::{self},
    io::{stdin, Write},
    path::PathBuf,
    rc::Rc,
};

use crate::{
    cmd_forget,
    common::{cmd::Runner, path::to_absolute_path},
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
        let current_path =
            env::current_dir().map_err(|e| miette!("Failed to get current directory: {}", e))?;

        let config_file = match name {
            Some(name) => {
                PathBuf::from(&self.config_path).join(format!("{}.yaml", name.sanitize()))
            }
            None => PathBuf::from(".laio.yaml"),
        };

        if let Some(copy_name) = copy {
            let source = PathBuf::from(&self.config_path).join(format!("{copy_name}.yaml"));

            fs::copy(&source, &config_file).map_err(|e| {
                miette!(
                    "Could not copy '{}' to '{}': {}",
                    source.display(),
                    config_file.display(),
                    e
                )
            })?;
        } else {
            let template = TEMPLATE
                .replace("{ name }", name.as_deref().unwrap_or("changeme"))
                .replace("{ path }", current_path.to_str().unwrap_or("."));

            // Write to the file without using a shell command
            let mut file = fs::File::create(&config_file)
                .map_err(|e| miette!("Could not create '{}': {}", config_file.display(), e))?;
            file.write_all(template.as_bytes())
                .map_err(|e| miette!("Could not write to '{}': {}", config_file.display(), e))?;
        }

        let editor = env::var("EDITOR").unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        self.cmd_runner
            .run(&cmd_forget!(&editor, args = [config_file]))
            .map_err(|e| miette!("Failed to open editor '{}': {}", &editor, e))
    }

    pub(crate) fn edit(&self, name: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_forget!(
            var("EDITOR").unwrap_or_else(|_| "vim".to_string()),
            args = [format!("{}/{}.yaml", self.config_path, name.sanitize())]
        ))
    }

    pub(crate) fn link(&self, name: &str, file: &str) -> Result<()> {
        let source =
            to_absolute_path(file).wrap_err(format!("Failed to get absolute path for '{file}'"))?;
        let destination = format!("{}/{}.yaml", self.config_path, name);
        self.cmd_runner
            .run(&cmd_forget!("ln", args = ["-s", &source, &destination]))
            .wrap_err(format!(
                "Failed to link '{}' to '{}'",
                &source.to_string_lossy(),
                destination
            ))
    }

    pub(crate) fn validate(&self, name: &Option<String>, file: &str) -> Result<()> {
        let config = match name {
            Some(name) => format!("{}/{}.yaml", &self.config_path, name),
            None => PathBuf::from(&file)
                .canonicalize()
                .map_err(|_e| Error::msg(format!("Failed to read config: {file}.")))?
                .to_string_lossy()
                .into_owned(),
        };
        let _ = Session::from_config(&PathBuf::from(&config)).wrap_err("Validation error!")?;
        Ok(())
    }

    pub(crate) fn delete(&self, name: &str, force: bool) -> Result<()> {
        if !force {
            println!("Are you sure you want to delete {name}? [y/N]");
            let mut input = String::new();
            stdin().read_line(&mut input).into_diagnostic()?;
            if input.trim() != "y" {
                println!("Aborting.");
                return Ok(());
            }
        }
        let file = format!("{}/{}.yaml", &self.config_path, name.sanitize());
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
                // Try to parse each YAML file and extract the name field
                Session::from_config(&path)
                    .map(|session| session.name)
                    .map_err(|e| {
                        // Log warning for files that can't be parsed, but don't fail the entire list
                        eprintln!("Warning: Failed to parse '{}': {}", path.display(), e);
                        e
                    })
                    .ok()
            })
            .collect::<Vec<String>>();

        entries.sort();
        Ok(entries)
    }
}

pub trait ConfigNameExt {
    fn sanitize(&self) -> String;
}

impl ConfigNameExt for str {
    fn sanitize(&self) -> String {
        self.replace(" ", "-")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
}
