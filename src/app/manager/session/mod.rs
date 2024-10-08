use anyhow::{anyhow, bail, Result};
use inquire::Select;
use std::{env, fs, path::PathBuf};

use crate::{
    app::{
        cmd::Runner,
        config::{FlexDirection, Pane, Session},
        parser::parse,
        tmux::{target::Target, Client, Dimensions},
    },
    util::path::{find_config, home_dir, resolve_symlink, sanitize_path, to_absolute_path},
};

pub(crate) struct SessionManager<R: Runner> {
    pub config_path: String,
    tmux_client: Client<R>,
}

pub(crate) const LAIO_CONFIG: &str = "LAIO_CONFIG";
pub(crate) const LOCAL_CONFIG: &str = ".laio.yaml";

impl<R: Runner> SessionManager<R> {
    pub(crate) fn new(config_path: &str, tmux_client: Client<R>) -> Self {
        Self {
            config_path: config_path.replace('~', env::var("HOME").unwrap().as_str()),
            tmux_client,
        }
    }

    pub(crate) fn start(
        &self,
        name: &Option<String>,
        file: &Option<String>,
        show_picker: bool,
        skip_startup_cmds: bool,
        skip_attach: bool,
    ) -> Result<()> {
        let config = match name {
            Some(name) => {
                to_absolute_path(&format!("{}/{}.yaml", &self.config_path, name).to_string())?
            }
            None => match file {
                Some(file) => to_absolute_path(file)?,
                None => match self.select_config(show_picker)? {
                    Some(config) => config,
                    None => return Err(anyhow::anyhow!("No configuration selected!")),
                },
            },
        };

        // handling session switches for sessions not managed by laio
        if name.is_some() && self.try_switch(name.as_ref().unwrap(), skip_attach)? {
            return Ok(());
        }

        let session = Session::from_config(&resolve_symlink(&config)?)?;

        // handling session switches managed by laio
        if self.try_switch(&session.name, skip_attach)? {
            return Ok(());
        }

        let dimensions = self.tmux_client.get_dimensions()?;

        if !skip_startup_cmds {
            self.run_startup_commands(&session)?;
        }

        let path = session
            .windows
            .first()
            .and_then(|window| window.first_leaf_path())
            .map(|path| sanitize_path(path, &session.path))
            .unwrap_or(session.path.clone());

        self.tmux_client.create_session(&session.name, &path)?;
        self.tmux_client.setenv(
            &Target::new(&session.name),
            LAIO_CONFIG,
            config.to_str().unwrap(),
        );

        self.tmux_client.flush_commands()?;

        self.process_windows(&session, &dimensions, skip_startup_cmds)?;

        self.tmux_client.bind_key(
            "prefix M-l",
            "display-popup -w 50 -h 16 -E \"laio start --show-picker \"",
        )?;

        self.tmux_client.flush_commands()?;

        if !skip_attach {
            if self.tmux_client.is_inside_session() {
                self.tmux_client.switch_client(session.name.as_str())?;
            } else {
                self.tmux_client.attach_session(session.name.as_str())?;
            }
        }

        Ok(())
    }

    pub(crate) fn stop(
        &self,
        name: &Option<String>,
        skip_shutdown_cmds: bool,
        stop_all: bool,
    ) -> Result<()> {
        let current_session_name = self.tmux_client.current_session_name()?;
        log::trace!("Current session name: {}", current_session_name);

        if !stop_all && name.is_none() && !self.tmux_client.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if stop_all && name.is_some() {
            bail!("Stopping all and specifying a session name are mutually exclusive.")
        };

        if stop_all {
            // stops all other laio sessions
            log::trace!("Closing all laio sessions.");
            for name in self.list()?.into_iter() {
                if name == current_session_name {
                    log::trace!("Skipping current session: {:?}", current_session_name);
                    continue;
                };

                if self.is_laio_session(&name)? {
                    log::trace!("Closing session: {:?}", name);
                    self.stop(&Some(name.to_string()), skip_shutdown_cmds, false)?;
                }
            }
            if !self.tmux_client.is_inside_session() {
                log::debug!("Not inside a session");
                return Ok(());
            }
        };

        let name = name.clone().unwrap_or(current_session_name.to_string());
        if !self.tmux_client.session_exists(&name) {
            bail!("Session {} does not exist!", &name);
        }
        if !self.is_laio_session(&name)? {
            log::debug!("Not a laio session: {}", &name);
            return Ok(());
        }

        let result = (|| -> Result<()> {
            if !skip_shutdown_cmds {
                // checking if session is managed by laio
                match self.tmux_client.getenv(&Target::new(&name), LAIO_CONFIG) {
                    Ok(config) => {
                        log::trace!("Config: {:?}", config);

                        let session =
                            Session::from_config(&resolve_symlink(&to_absolute_path(&config)?)?)?;
                        self.run_shutdown_commands(&session)
                    }
                    Err(e) => {
                        log::warn!("LAIO_CONFIG environment variable not found: {:?}", e);
                        Ok(())
                    }
                }
            } else {
                log::trace!("Skipping shutdown commands for session: {:?}", name);
                Ok(())
            }
        })();

        let stop_result = self
            .tmux_client
            .stop_session(name.as_str())
            .map_err(Into::into);

        result.and(stop_result)
    }

    pub(crate) fn list(&self) -> Result<Vec<String>> {
        self.tmux_client.list_sessions()
    }

    pub(crate) fn to_yaml(&self) -> Result<String> {
        let home_dir = home_dir()?;
        let layout: String = self.tmux_client.session_layout()?;
        let name: String = self.tmux_client.session_name()?;
        let path: String = self
            .tmux_client
            .session_start_path()?
            .replace(&home_dir, "~");
        let pane_paths = self.tmux_client.pane_paths()?;

        log::trace!("session_to_yaml: {}", layout);

        let tokens = parse(&layout, &pane_paths, &path);
        log::trace!("tokens: {:#?}", tokens);

        let session = Session::from_tokens(&name, &path, &tokens);
        log::trace!("session: {:#?}", session);

        let yaml = serde_yaml::to_string(&session)?;

        Ok(yaml)
    }

    pub(crate) fn is_laio_session(&self, name: &str) -> Result<bool> {
        Ok(self
            .tmux_client
            .getenv(&Target::new(name), LAIO_CONFIG)
            .is_ok())
    }

    fn process_windows(
        &self,
        session: &Session,
        dimensions: &Dimensions,
        skip_cmds: bool,
    ) -> Result<()> {
        let base_idx = self.tmux_client.get_base_idx()?;
        log::trace!("base-index: {}", base_idx);

        session
            .windows
            .iter()
            .enumerate()
            .try_for_each(|(i, window)| -> Result<()> {
                let idx = i + base_idx;

                // create or rename window
                let window_id = if idx == base_idx {
                    let id = self.tmux_client.get_current_window(&session.name)?;
                    self.tmux_client
                        .rename_window(&Target::new(&session.name).window(&id), &window.name)?;
                    id
                } else {
                    let path = sanitize_path(
                        window.first_leaf_path().unwrap_or(&"".to_string()),
                        &session.path,
                    );

                    self.tmux_client
                        .new_window(&session.name, &window.name, &path)?
                };
                log::trace!("window-id: {}", window_id);

                // apply layout to window
                self.tmux_client.select_custom_layout(
                    &Target::new(&session.name).window(&window_id),
                    &self.generate_layout(
                        &LayoutMeta {
                            name: session.name.as_str(),
                            id: window_id.as_str(),
                            path: session.path.as_str(),
                        },
                        &LayoutInfo {
                            dimensions,
                            direction: &window.flex_direction,
                            xy: (0, 0),
                        },
                        &window.panes,
                        skip_cmds,
                        0,
                    )?,
                )?;

                Ok(())
            })
    }

    fn run_startup_commands(&self, session: &Session) -> Result<()> {
        log::info!("Running startup commands...");
        self.run_session_commands(&session.startup, &session.path)?;
        log::info!("Completed startup commands.");
        Ok(())
    }

    fn run_shutdown_commands(&self, session: &Session) -> Result<()> {
        log::info!("Running shutdown commands...");
        self.run_session_commands(&session.shutdown, &session.path)?;
        log::info!("Completed shutdown commands.");
        Ok(())
    }

    fn run_session_commands(&self, commands: &[String], session_path: &String) -> Result<()> {
        if commands.is_empty() {
            return Ok(());
        }

        log::info!("Running commands...");

        // Save the current directory to restore it later
        let current_dir = env::current_dir()?;

        log::trace!("Current directory: {:?}", current_dir);
        log::trace!("Changing to: {:?}", session_path);

        // Use to_absolute_path to handle the session path
        env::set_current_dir(to_absolute_path(session_path)?)
            .map_err(|_| anyhow!("Unable to change to directory: {:?}", &session_path))?;

        // Run each command
        for cmd in commands {
            self.tmux_client
                .run_session_command(cmd)
                .map_err(|_| anyhow!("Failed to run command: {}", cmd))?;
        }

        // Restore the original directory
        env::set_current_dir(&current_dir)
            .map_err(|_| anyhow!("Failed to restore original directory {:?}", current_dir))?;

        log::info!("Completed commands.");

        Ok(())
    }

    fn generate_layout(
        &self,
        layout_meta: &LayoutMeta,
        layout_info: &LayoutInfo,
        panes: &[Pane],
        skip_cmds: bool,
        depth: usize,
    ) -> Result<String> {
        let flex_total = panes.iter().map(|p| p.flex).sum();

        let (mut current_x, mut current_y) = layout_info.xy;

        let mut pane_strings: Vec<String> = Vec::new();
        let mut num_dividers = 0;

        let session_name = layout_meta.name;
        let window_path = layout_meta.path;
        let window_id = layout_meta.id;

        for (index, pane) in panes.iter().enumerate() {
            let (pane_width, pane_height, next_x, next_y) = match self.calculate_pane_dimensions(
                &LayoutInfo {
                    dimensions: layout_info.dimensions,
                    direction: layout_info.direction,
                    xy: (current_x, current_y),
                },
                &CalculateInfo {
                    depth,
                    dividers: num_dividers,
                    flex: pane.flex,
                    index,
                    flex_total,
                },
                panes,
            ) {
                Some(value) => value,
                None => continue,
            };

            // Increment divider count after calculating position and dimension for this pane
            if depth > 0 || index > 0 {
                num_dividers += 1;
            }

            // Create panes in tmux as we go
            let pane_id = if index > 0 {
                let path = sanitize_path(
                    pane.first_leaf_path().unwrap_or(&".".to_string()),
                    &window_path.to_string(),
                );
                self.tmux_client
                    .split_window(&Target::new(session_name).window(window_id), &path)?
            } else {
                self.tmux_client
                    .get_current_pane(&Target::new(session_name).window(window_id))?
            };

            if pane.zoom {
                self.tmux_client.zoom_pane(
                    &Target::new(session_name)
                        .window(window_id)
                        .pane(pane_id.as_str()),
                );
            };

            // apply styles to pane if it has any
            if let Some(style) = &pane.style {
                self.tmux_client.set_pane_style(
                    &Target::new(session_name)
                        .window(window_id)
                        .pane(pane_id.as_str()),
                    style,
                )?;
            }

            self.tmux_client
                .select_layout(&Target::new(session_name).window(window_id), "tiled")?;

            // Push the determined string into pane_strings
            pane_strings.push(self.generate_pane_string(
                layout_meta,
                &LayoutInfo {
                    dimensions: &Dimensions {
                        width: pane_width,
                        height: pane_height,
                    },
                    direction: layout_info.direction,
                    xy: (current_x, current_y),
                },
                pane,
                depth,
                &pane_id,
                skip_cmds,
            )?);

            (current_x, current_y) = (next_x, next_y);
            if !skip_cmds {
                self.tmux_client.register_commands(
                    &Target::new(session_name)
                        .window(window_id)
                        .pane(pane_id.as_str()),
                    &pane.commands,
                );
            };
        }

        if pane_strings.len() > 1 {
            let (open_delimiter, close_delimiter) = match layout_info.direction {
                FlexDirection::Column => ('[', ']'),
                FlexDirection::Row => ('{', '}'),
            };

            Ok(format!(
                "{}x{},0,0{}{}{}",
                layout_info.dimensions.width,
                layout_info.dimensions.height,
                open_delimiter,
                pane_strings.join(","),
                close_delimiter
            ))
        } else {
            Ok(format!(
                "{}x{},0,0",
                layout_info.dimensions.width, layout_info.dimensions.height
            ))
        }
    }

    fn generate_pane_string(
        &self,
        layout_meta: &LayoutMeta,
        layout_info: &LayoutInfo,
        pane: &Pane,
        depth: usize,
        pane_id: &str,
        skip_cmds: bool,
    ) -> Result<String> {
        let pane_string = if !pane.panes.is_empty() {
            // Generate layout string for sub-panes
            self.generate_layout(
                layout_meta,
                &LayoutInfo {
                    dimensions: layout_info.dimensions,
                    direction: &pane.flex_direction,
                    xy: layout_info.xy,
                },
                &pane.panes,
                skip_cmds,
                depth + 1,
            )?
        } else {
            // Format string for the current pane
            let (current_x, current_y) = layout_info.xy;
            format!(
                "{0}x{1},{2},{3},{4}",
                layout_info.dimensions.width,
                layout_info.dimensions.height,
                current_x,
                current_y,
                pane_id.replace('%', "")
            )
        };
        Ok(pane_string)
    }

    fn calculate_pane_dimensions(
        &self,
        layout_info: &LayoutInfo,
        calculate_info: &CalculateInfo,
        panes: &[Pane],
    ) -> Option<(usize, usize, usize, usize)> {
        let (current_x, current_y) = layout_info.xy;
        let index = calculate_info.index;
        let (pane_width, pane_height, next_x, next_y) = match layout_info.direction {
            FlexDirection::Column => {
                let h = self.calculate_dimension(
                    calculate_info,
                    index == panes.len() - 1,
                    current_y,
                    layout_info.dimensions.height,
                )?;
                (
                    layout_info.dimensions.width,
                    h,
                    layout_info.xy.0,
                    layout_info.xy.1 + h + 1,
                )
            }
            _ => {
                let w = self.calculate_dimension(
                    calculate_info,
                    index == panes.len() - 1,
                    current_x,
                    layout_info.dimensions.width,
                )?;
                (
                    w,
                    layout_info.dimensions.height,
                    current_x + w + 1,
                    current_y,
                )
            }
        };
        Some((pane_width, pane_height, next_x, next_y))
    }

    fn calculate_dimension(
        &self,
        calculate_info: &CalculateInfo,
        is_last_pane: bool,
        current_value: usize,
        total_value: usize,
    ) -> Option<usize> {
        let (flex, flex_total, dividers, depth, index) = (
            calculate_info.flex,
            calculate_info.flex_total,
            calculate_info.dividers,
            calculate_info.depth,
            calculate_info.index,
        );
        if is_last_pane {
            log::trace!(
                "current_value: {}, total_value: {}",
                current_value,
                total_value
            );
            if current_value >= total_value {
                log::warn!(
                    "skipping pane: total_value: {}, current_value: {}",
                    total_value,
                    current_value
                );
                return None;
            }
            Some(total_value - current_value) // Give the remaining value to the last pane
        } else {
            // Calculate based on flex, total flex, and dividers
            Some(if depth > 0 || index > 0 {
                total_value * flex / flex_total - dividers
            } else {
                total_value * flex / flex_total
            })
        }
    }

    fn try_switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        if self.tmux_client.session_exists(name) {
            log::warn!("Session '{}' already exists", name);
            if !skip_attach {
                if self.tmux_client.is_inside_session() {
                    self.tmux_client.switch_client(name)?;
                } else {
                    self.tmux_client.attach_session(name)?;
                }
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn select_config(&self, show_picker: bool) -> Result<Option<PathBuf>> {
        fn picker(config_path: &str, sessions: &Vec<String>) -> Result<Option<PathBuf>> {
            let configs = fs::read_dir(config_path)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
                .filter_map(|path| {
                    path.file_stem()
                        .and_then(|name| name.to_str())
                        .map(String::from)
                })
                .collect::<Vec<String>>();

            let mut merged: Vec<String> = sessions
                .iter()
                .map(|s| {
                    if configs.contains(s) {
                        format!("{} *", s)
                    } else {
                        s.to_string()
                    }
                })
                .collect();

            merged.extend(
                configs
                    .iter()
                    .filter(|s| !sessions.contains(s))
                    .map(|s| s.to_string()),
            );

            merged.sort();
            merged.dedup();

            let selected = Select::new("Select configuration:", merged)
                .with_page_size(12)
                .prompt();

            match selected {
                Ok(config) => Ok(Some(PathBuf::from(format!(
                    "{}/{}.yaml",
                    &config_path, config.trim_end_matches(" *")
                )))),
                Err(_) => Ok(None),
            }
        }

        if show_picker {
            picker(&self.config_path, &self.list()?)
        } else {
            match find_config(&to_absolute_path(LOCAL_CONFIG)?) {
                Ok(config) => Ok(Some(config)),
                Err(err) => {
                    log::debug!("{}", err);
                    picker(&self.config_path, &self.list()?)
                }
            }
        }
    }
}

struct LayoutInfo<'a> {
    dimensions: &'a Dimensions,
    direction: &'a FlexDirection,
    xy: (usize, usize),
}

struct LayoutMeta<'a> {
    id: &'a str,
    name: &'a str,
    path: &'a str,
}

struct CalculateInfo {
    depth: usize,
    dividers: usize,
    flex: usize,
    flex_total: usize,
    index: usize,
}

#[cfg(test)]
mod test;
