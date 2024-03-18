use anyhow::{anyhow, bail, Error, Result};
use std::{env, rc::Rc};

use crate::{
    app::{
        cmd::CmdRunner,
        cmd::CommandType,
        config::{FlexDirection, Pane, Session},
        parser::parse,
        tmux::{Dimensions, Tmux},
    },
    cmd_basic, cmd_verbose,
    util::path::{resolve_symlink, sanitize_path, to_absolute_path},
};

pub(crate) struct SessionManager<R: CmdRunner> {
    pub config_path: String,
    cmd_runner: Rc<R>,
}

impl<R: CmdRunner> SessionManager<R> {
    pub(crate) fn new(config_path: &str, cmd_runner: Rc<R>) -> Self {
        Self {
            config_path: config_path.replace("~", env::var("HOME").unwrap().as_str()),
            cmd_runner,
        }
    }

    pub(crate) fn start(
        &self,
        name: &Option<String>,
        file: &str,
        skip_startup_cmds: &bool,
    ) -> Result<(), Error> {
        let config = match name {
            Some(name) => format!("{}/{}.yaml", &self.config_path, name),
            None => file.to_string(),
        };

        // handling session switches for sessions not managed by laio
        if name.is_some() && self.try_switch(name.as_ref().unwrap(), None)? {
            return Ok(());
        }

        let session = Session::from_config(&resolve_symlink(&to_absolute_path(&config)?)?)?;

        // create tmux client
        let tmux = Tmux::new(
            &Some(session.name.clone()),
            &session.path,
            Rc::clone(&self.cmd_runner),
        );

        // handling session switches managed by laio
        if self.try_switch(&session.name, Some(&tmux))? {
            return Ok(());
        }

        let dimensions = tmux.get_dimensions()?;

        if !*skip_startup_cmds {
            self.run_startup_commands(&session)?;
        }

        tmux.create_session(&config)?;

        self.process_windows(&session, &tmux, &dimensions)?;

        tmux.bind_key("prefix s", "display-popup -E \"SESSION=\\$(laio ls | fzf --exit-0 | sed 's/\\*$//') && laio start \\$SESSION\"")?;

        tmux.flush_commands()?;

        if tmux.is_inside_session() {
            tmux.switch_client()?;
        } else {
            tmux.attach_session()?;
        }

        Ok(())
    }

    pub(crate) fn stop(
        &self,
        name: &Option<String>,
        skip_shutdown_cmds: &bool,
    ) -> Result<(), Error> {
        let tmux = Tmux::new(name, &".".to_string(), Rc::clone(&self.cmd_runner));

        if let Some(ref session_name) = name {
            if !tmux.session_exists(session_name) {
                bail!("Session {} does not exist!", session_name);
            }
        }

        let result = (|| -> Result<(), Error> {
            if !*skip_shutdown_cmds {
                // checking if session is managed by laio
                match tmux.getenv("", "LAIO_CONFIG") {
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
                Ok(())
            }
        })();

        let stop_result = tmux
            .stop_session(name.as_deref().unwrap_or(""))
            .map_err(Into::into);

        result.and(stop_result)
    }

    pub(crate) fn list(&self) -> Result<Vec<String>, Error> {
        Ok(Tmux::new(&None, &".".to_string(), Rc::clone(&self.cmd_runner)).list_sessions()?)
    }

    pub(crate) fn to_yaml(&self) -> Result<String, Error> {
        let res: String = self.cmd_runner.run(&cmd_basic!(
            "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\""
        ))?;
        let name: String = self
            .cmd_runner
            .run(&cmd_basic!("tmux display-message -p \"#S\""))?;

        log::trace!("session_to_yaml: {}", res);

        let tokens = parse(&res);
        log::trace!("tokens: {:#?}", tokens);

        let session = Session::from_tokens(&name, &tokens);
        log::trace!("session: {:#?}", session);

        let yaml = serde_yaml::to_string(&session)?;

        Ok(yaml)
    }

    fn process_windows(
        &self,
        session: &Session,
        tmux: &Tmux<R>,
        dimensions: &Dimensions,
    ) -> Result<(), Error> {
        let base_idx = tmux.get_base_idx()?;
        log::trace!("base-index: {}", base_idx);

        session
            .windows
            .iter()
            .enumerate()
            .try_for_each(|(i, window)| -> Result<(), Error> {
                let idx = i + base_idx;

                // create new window
                let window_id = tmux.new_window(&window.name, &session.path)?;
                log::trace!("window-id: {}", window_id);

                // delete first window and move others
                if idx == base_idx {
                    tmux.delete_window(base_idx)?;
                    tmux.move_windows()?;
                }

                // apply layout to window
                tmux.select_custom_layout(
                    &window_id,
                    &self.generate_layout(
                        &window_id,
                        &session.path,
                        &window.panes,
                        dimensions.width,
                        dimensions.height,
                        &window.flex_direction,
                        0,
                        0,
                        tmux,
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
        env::set_current_dir(to_absolute_path(&session_path)?)
            .map_err(|_| anyhow!("Unable to change to directory: {:?}", &session_path))?;

        // Run each command
        for cmd in commands {
            self.cmd_runner
                .run(&cmd_verbose!("{}", cmd))
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
        window_id: &str,
        window_path: &str,
        panes: &[Pane],
        width: usize,
        height: usize,
        direction: &FlexDirection,
        start_x: usize,
        start_y: usize,
        tmux: &Tmux<R>,
        depth: usize,
    ) -> Result<String, Error> {
        let total_flex = panes.iter().map(|p| p.flex).sum();

        let mut current_x = start_x;
        let mut current_y = start_y;

        let mut pane_strings: Vec<String> = Vec::new();
        let mut dividers = 0;

        for (index, pane) in panes.iter().enumerate() {
            let (pane_width, pane_height, next_x, next_y) = match self.calculate_pane_dimensions(
                direction, index, panes, width, current_x, depth, pane.flex, total_flex, dividers,
                height, current_y,
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
                tmux.split_window(window_id, &path)?
            } else {
                tmux.get_current_pane(window_id)?
            };

            if index == 0 {
                tmux.register_commands(
                    &format!("{}.{}", window_id, pane_id),
                    &vec![format!("cd \"{}\"", &path)],
                );
            };

            // apply styles to pane if it has any
            if let Some(style) = &pane.style {
                tmux.set_pane_style(&format!("{}.{}", window_id, pane_id), style)?;
            }

            tmux.select_layout(window_id, &"tiled".to_string())?;

            // Push the determined string into pane_strings
            pane_strings.push(self.generate_pane_string(
                pane,
                window_id,
                window_path,
                pane_width,
                pane_height,
                current_x,
                current_y,
                tmux,
                depth,
                &pane_id,
            )?);

            current_x = next_x;
            current_y = next_y;
            tmux.register_commands(&format!("{}.{}", window_id, pane_id), &pane.commands);
        }

        if pane_strings.len() > 1 {
            match direction {
                FlexDirection::Column => Ok(format!(
                    "{}x{},0,0[{}]",
                    width,
                    height,
                    pane_strings.join(",")
                )),
                _ => Ok(format!(
                    "{}x{},0,0{{{}}}",
                    width,
                    height,
                    pane_strings.join(",")
                )),
            }
        } else {
            Ok(format!("{}x{},0,0", width, height))
        }
    }

    fn generate_pane_string(
        &self,
        pane: &Pane,
        window_id: &str,
        window_path: &str,
        pane_width: usize,
        pane_height: usize,
        current_x: usize,
        current_y: usize,
        tmux: &Tmux<R>,
        depth: usize,
        pane_id: &String,
    ) -> Result<String, Error> {
        let pane_string = if let Some(sub_panes) = &pane.panes {
            // Generate layout string for sub-panes
            self.generate_layout(
                window_id,
                window_path,
                sub_panes,
                pane_width,
                pane_height,
                &pane.flex_direction,
                current_x,
                current_y,
                &tmux,
                depth + 1,
            )?
        } else {
            // Format string for the current pane
            format!(
                "{0}x{1},{2},{3},{4}",
                pane_width,
                pane_height,
                current_x,
                current_y,
                pane_id.replace("%", "")
            )
        };
        Ok(pane_string)
    }

    fn calculate_pane_dimensions(
        &self,
        direction: &FlexDirection,
        index: usize,
        panes: &[Pane],
        width: usize,
        current_x: usize,
        depth: usize,
        flex: usize,
        total_flex: usize,
        dividers: usize,
        height: usize,
        current_y: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let (pane_width, pane_height, next_x, next_y) = match direction {
            FlexDirection::Column => {
                let h = self.calculate_dimension(
                    index == panes.len() - 1,
                    current_y,
                    height,
                    flex,
                    total_flex,
                    dividers,
                    depth,
                    index,
                )?;
                (width, h, current_x, current_y + h + 1)
            }
            _ => {
                let w = self.calculate_dimension(
                    index == panes.len() - 1,
                    current_x,
                    width,
                    flex,
                    total_flex,
                    dividers,
                    depth,
                    index,
                )?;
                (w, height, current_x + w + 1, current_y)
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

    fn try_switch(&self, name: &str, tmux_option: Option<&Tmux<R>>) -> Result<bool> {
        let tmux_owned;
        let tmux_ref;

        if let Some(tmux) = tmux_option {
            tmux_ref = tmux;
        } else {
            tmux_owned = Tmux::new(
                &Some(name.to_string()), // Assuming Tmux::new expects an Option<String>
                &"".to_string(),
                Rc::clone(&self.cmd_runner),
            );
            tmux_ref = &tmux_owned;
        }

        if tmux_ref.session_exists(name) {
            log::warn!("Session '{}' already exists", name);
            if tmux_ref.is_inside_session() {
                tmux_ref.switch_client()?;
            } else {
                tmux_ref.attach_session()?;
            }
            return Ok(true);
        }

        Ok(false)
    }

    #[cfg(test)]
    pub(crate) fn cmd_runner(&self) -> &R {
        &self.cmd_runner
    }
}
