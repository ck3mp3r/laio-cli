use anyhow::{anyhow, bail, Error, Result};
use std::env;

use crate::{
    app::{
        cmd::Runner,
        config::{FlexDirection, Pane, Session},
        parser::parse,
        tmux::{Client, Dimensions},
    },
    util::path::{resolve_symlink, sanitize_path, to_absolute_path},
};

pub(crate) struct SessionManager<R: Runner> {
    pub config_path: String,
    tmux_client: Client<R>,
}

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
        file: &str,
        skip_startup_cmds: &bool,
        skip_attach: &bool,
    ) -> Result<(), Error> {
        let config = match name {
            Some(name) => format!("{}/{}.yaml", &self.config_path, name),
            None => file.to_string(),
        };

        // handling session switches for sessions not managed by laio
        if name.is_some() && self.try_switch(name.as_ref().unwrap(), skip_attach)? {
            return Ok(());
        }

        let session = Session::from_config(&resolve_symlink(&to_absolute_path(&config)?)?)?;

        // handling session switches managed by laio
        if self.try_switch(&session.name, skip_attach)? {
            return Ok(());
        }

        let dimensions = self.tmux_client.get_dimensions()?;

        if !*skip_startup_cmds {
            self.run_startup_commands(&session)?;
        }

        let path = session
            .windows
            .first()
            .and_then(|window| window.panes.first())
            .and_then(|pane| Some(sanitize_path(&pane.path, &session.path)))
            .unwrap_or(session.path.clone());

        self.tmux_client
            .create_session(&session.name, &path, &config)?;
        self.tmux_client.flush_commands()?;

        self.process_windows(&session, &dimensions, skip_startup_cmds)?;

        self.tmux_client.bind_key("prefix M-l", "display-popup -E \"SESSION=\\\"\\$(laio ls | fzf --exit-0 | sed 's/ \\{0,1\\}\\*$//')\\\" && if [ -n \\\"\\$SESSION\\\" ]; then laio start \\\"\\$SESSION\\\"; fi\"")?;

        self.tmux_client.flush_commands()?;

        if !*skip_attach {
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
        skip_shutdown_cmds: &bool,
        stop_all: &bool,
    ) -> Result<(), Error> {
        let current_session_name = self.tmux_client.current_session_name()?;
        log::trace!("Current session name: {}", current_session_name);

        if !*stop_all && name.is_none() && !self.tmux_client.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if *stop_all && name.is_some() {
            bail!("Stopping all and specifying a session name are mutually exclusive.")
        };

        if *stop_all {
            // stops all other laio sessions
            log::trace!("Closing all laio sessions.");
            for name in self.list()?.into_iter() {
                if name == current_session_name {
                    log::trace!("Skipping current session: {:?}", current_session_name);
                    continue;
                };

                if self.is_laio_session(&name)? {
                    log::trace!("Closing session: {:?}", name);
                    self.stop(&Some(name.to_string()), skip_shutdown_cmds, &false)?;
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

        let result = (|| -> Result<(), Error> {
            if !*skip_shutdown_cmds {
                // checking if session is managed by laio
                match self.tmux_client.getenv(&name, "", "LAIO_CONFIG") {
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

    pub(crate) fn list(&self) -> Result<Vec<String>, Error> {
        self.tmux_client.list_sessions()
    }

    pub(crate) fn to_yaml(&self) -> Result<String, Error> {
        let res: String = self.tmux_client.session_layout()?;
        let name: String = self.tmux_client.session_name()?;

        log::trace!("session_to_yaml: {}", res);

        let tokens = parse(&res);
        log::trace!("tokens: {:#?}", tokens);

        let session = Session::from_tokens(&name, &tokens);
        log::trace!("session: {:#?}", session);

        let yaml = serde_yaml::to_string(&session)?;

        Ok(yaml)
    }

    pub(crate) fn is_laio_session(&self, name: &str) -> Result<bool> {
        Ok(self.tmux_client.getenv(name, "", "LAIO_CONFIG").is_ok())
    }

    fn process_windows(
        &self,
        session: &Session,
        dimensions: &Dimensions,
        skip_cmds: &bool,
    ) -> Result<(), Error> {
        let base_idx = self.tmux_client.get_base_idx()?;
        log::trace!("base-index: {}", base_idx);

        session
            .windows
            .iter()
            .enumerate()
            .try_for_each(|(i, window)| -> Result<(), Error> {
                let idx = i + base_idx;

                // create or rename window
                let window_id = if idx == base_idx {
                    let id = self.tmux_client.get_current_window(&session.name)?;
                    self.tmux_client
                        .rename_window(&session.name, &id, &window.name)?;
                    id
                } else {
                    let path = window
                        .panes
                        .first()
                        .and_then(|pane| Some(sanitize_path(&pane.path, &session.path)))
                        .unwrap_or(session.path.clone());

                    self.tmux_client
                        .new_window(&session.name, &window.name, &path)?
                };
                log::trace!("window-id: {}", window_id);

                // apply layout to window
                self.tmux_client.select_custom_layout(
                    &session.name,
                    &window_id,
                    &self.generate_layout(
                        session,
                        &window_id,
                        &session.path,
                        &window.panes,
                        dimensions,
                        &window.flex_direction,
                        (0, 0),
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
        session: &Session,
        window_id: &str,
        window_path: &str,
        panes: &[Pane],
        dimensions: &Dimensions,
        direction: &FlexDirection,
        start_xy: (usize, usize),
        skip_cmds: &bool,
        depth: usize,
    ) -> Result<String, Error> {
        let total_flex = panes.iter().map(|p| p.flex).sum();

        let (mut current_x, mut current_y) = start_xy;

        let mut pane_strings: Vec<String> = Vec::new();
        let mut dividers = 0;

        for (index, pane) in panes.iter().enumerate() {
            let (pane_width, pane_height, next_x, next_y) = match self.calculate_pane_dimensions(
                direction,
                index,
                panes,
                dimensions,
                (current_x, current_y),
                depth,
                pane.flex,
                total_flex,
                dividers,
            ) {
                Some(value) => value,
                None => continue,
            };

            // Increment divider count after calculating position and dimension for this pane
            if depth > 0 || index > 0 {
                dividers += 1;
            }

            let path = sanitize_path(&pane.path, &window_path.to_string());

            // Create panes in tmux as we go
            let pane_id = if index > 0 {
                self.tmux_client
                    .split_window(&session.name, window_id, &path)?
            } else {
                self.tmux_client
                    .get_current_pane(&session.name, window_id)?
            };

            if pane.zoom {
                self.tmux_client
                    .zoom_pane(&session.name, &format!("{}.{}", window_id, pane_id));
            };

            // apply styles to pane if it has any
            if let Some(style) = &pane.style {
                self.tmux_client.set_pane_style(
                    &session.name,
                    &format!("{}.{}", window_id, pane_id),
                    style,
                )?;
            }

            self.tmux_client
                .select_layout(&session.name, window_id, "tiled")?;

            // Push the determined string into pane_strings
            pane_strings.push(self.generate_pane_string(
                session,
                pane,
                window_id,
                window_path,
                &Dimensions {
                    width: pane_width,
                    height: pane_height,
                },
                (current_x, current_y),
                depth,
                &pane_id,
                skip_cmds,
            )?);

            (current_x, current_y) = (next_x, next_y);
            if !skip_cmds {
                self.tmux_client.register_commands(
                    &session.name,
                    &format!("{}.{}", window_id, pane_id),
                    &pane.commands,
                );
            };
        }

        if pane_strings.len() > 1 {
            match direction {
                FlexDirection::Column => Ok(format!(
                    "{}x{},0,0[{}]",
                    dimensions.width,
                    dimensions.height,
                    pane_strings.join(",")
                )),
                _ => Ok(format!(
                    "{}x{},0,0{{{}}}",
                    dimensions.width,
                    dimensions.height,
                    pane_strings.join(",")
                )),
            }
        } else {
            Ok(format!("{}x{},0,0", dimensions.width, dimensions.height))
        }
    }

    fn generate_pane_string(
        &self,
        session: &Session,
        pane: &Pane,
        window_id: &str,
        window_path: &str,
        dimensions: &Dimensions,
        current_xy: (usize, usize),
        depth: usize,
        pane_id: &str,
        skip_cmds: &bool,
    ) -> Result<String, Error> {
        let pane_string = if let Some(sub_panes) = &pane.panes {
            // Generate layout string for sub-panes
            self.generate_layout(
                session,
                window_id,
                window_path,
                sub_panes,
                dimensions,
                &pane.flex_direction,
                current_xy,
                skip_cmds,
                depth + 1,
            )?
        } else {
            // Format string for the current pane
            let (current_x, current_y) = current_xy;
            format!(
                "{0}x{1},{2},{3},{4}",
                dimensions.width,
                dimensions.height,
                current_x,
                current_y,
                pane_id.replace('%', "")
            )
        };
        Ok(pane_string)
    }

    fn calculate_pane_dimensions(
        &self,
        direction: &FlexDirection,
        index: usize,
        panes: &[Pane],
        dimensions: &Dimensions,
        current_xy: (usize, usize),
        depth: usize,
        flex: usize,
        total_flex: usize,
        dividers: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let (current_x, current_y) = current_xy;
        let (pane_width, pane_height, next_x, next_y) = match direction {
            FlexDirection::Column => {
                let h = self.calculate_dimension(
                    index == panes.len() - 1,
                    current_y,
                    dimensions.height,
                    flex,
                    total_flex,
                    dividers,
                    depth,
                    index,
                )?;
                (dimensions.width, h, current_xy.0, current_xy.1 + h + 1)
            }
            _ => {
                let w = self.calculate_dimension(
                    index == panes.len() - 1,
                    current_x,
                    dimensions.width,
                    flex,
                    total_flex,
                    dividers,
                    depth,
                    index,
                )?;
                (w, dimensions.height, current_x + w + 1, current_y)
            }
        };
        Some((pane_width, pane_height, next_x, next_y))
    }

    fn calculate_dimension(
        &self,
        is_last_pane: bool,
        current_value: usize,
        total_value: usize,
        flex: usize,
        total_flex: usize,
        dividers: usize,
        depth: usize,
        index: usize,
    ) -> Option<usize> {
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
                total_value * flex / total_flex - dividers
            } else {
                total_value * flex / total_flex
            })
        }
    }

    fn try_switch(&self, name: &str, skip_attach: &bool) -> Result<bool> {
        if self.tmux_client.session_exists(name) {
            log::warn!("Session '{}' already exists", name);
            if !*skip_attach {
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
}

#[cfg(test)]
mod test;
