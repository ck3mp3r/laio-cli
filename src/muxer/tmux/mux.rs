use std::sync::Arc;

use miette::{bail, Result};
use tokio::runtime::Runtime;

use crate::{
    app::manager::session::manager::LAIO_CONFIG,
    common::{
        cmd::{Runner, ShellRunner},
        config::{FlexDirection, Pane, Session},
        muxer::{Client, Multiplexer},
        path::{home_dir, resolve_symlink, sanitize_path, to_absolute_path},
        session_info::SessionInfo,
    },
    muxer::tmux::parser::parse,
    tmux_target,
};

use super::{client::TmuxClient, Dimensions, Target};

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

pub(crate) struct Tmux<R: Runner = ShellRunner> {
    client: TmuxClient<R>,
    runtime: Runtime,
}

impl Tmux {
    pub fn new() -> Self {
        Self::new_with_runner(ShellRunner::new())
    }
}

impl<R: Runner> Tmux<R> {
    pub fn new_with_runner(runner: R) -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        Self {
            client: TmuxClient::new(Arc::new(runner)),
            runtime,
        }
    }

    fn process_windows(
        &self,
        session: &Session,
        dimensions: &Dimensions,
        skip_cmds: bool,
    ) -> Result<()> {
        let base_idx = self.client.get_base_idx()?;
        log::trace!("base-index: {base_idx}");

        session
            .windows
            .iter()
            .enumerate()
            .try_for_each(|(i, window)| -> Result<()> {
                let idx = i + base_idx;

                let window_id = if idx == base_idx {
                    let id = self.client.get_current_window(&session.name)?;
                    self.client
                        .rename_window(&tmux_target!(&session.name, &id), &window.name)?;
                    id
                } else {
                    let path = sanitize_path(
                        window.first_leaf_path().unwrap_or(&"".to_string()),
                        &session.path,
                    );

                    self.client.new_window(&session.name, &window.name, &path)?
                };
                log::trace!("window-id: {window_id}");

                self.client.select_custom_layout(
                    &tmux_target!(&session.name, &window_id),
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
            log::trace!("current_value: {current_value}, total_value: {total_value}");
            if current_value >= total_value {
                log::warn!(
                    "skipping pane: total_value: {total_value}, current_value: {current_value}"
                );
                return None;
            }
            Some(total_value - current_value)
        } else {
            Some(if depth > 0 || index > 0 {
                total_value * flex / flex_total - dividers
            } else {
                total_value * flex / flex_total
            })
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

            if depth > 0 || index > 0 {
                num_dividers += 1;
            }

            let pane_id = if index > 0 {
                let path = sanitize_path(
                    pane.first_leaf_path().unwrap_or(&".".to_string()),
                    &window_path.to_string(),
                );
                self.client
                    .split_window(&tmux_target!(session_name, window_id), &path)?
            } else {
                self.client
                    .get_current_pane(&tmux_target!(session_name, window_id))?
            };

            if let Some(name) = &pane.name {
                self.client.set_pane_title(
                    &tmux_target!(session_name, window_id, pane_id.as_str()),
                    name.as_str(),
                );
            };

            if pane.zoom {
                self.client
                    .zoom_pane(&tmux_target!(session_name, window_id, pane_id.as_str()));
            };

            if pane.focus {
                self.client
                    .focus_pane(&tmux_target!(session_name, window_id, pane_id.as_str()));
            };

            if let Some(style) = &pane.style {
                self.client.set_pane_style(
                    &tmux_target!(session_name, window_id, pane_id.as_str()),
                    style,
                )?;
            }

            self.client
                .select_layout(&tmux_target!(session_name, window_id), "tiled")?;

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
                let mut commands = pane.commands.clone();
                if let Some(script) = &pane.script {
                    commands.push(script.to_cmd()?);
                }
                self.client.register_commands(
                    &tmux_target!(session_name, window_id, pane_id.as_str()),
                    &commands,
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

    fn is_laio_session(&self, name: &str) -> Result<bool> {
        Ok(self.client.getenv(&tmux_target!(name), LAIO_CONFIG).is_ok())
    }
}

impl<R: Runner> Multiplexer for Tmux<R> {
    fn start(
        &self,
        session: &Session,
        config: &str,
        skip_attach: bool,
        skip_cmds: bool,
    ) -> Result<()> {
        if self.switch(&session.name, skip_attach)? {
            return Ok(());
        }

        let dimensions = self.client.get_dimensions()?;

        if !skip_cmds {
            let mut commands = session.startup.clone();
            if let Some(script) = &session.startup_script {
                commands.push(script.to_cmd()?);
            }

            self.client.run_commands(&commands, &session.path)?;
        }

        let path = session
            .windows
            .first()
            .and_then(|window| window.first_leaf_path())
            .map(|path| sanitize_path(path, &session.path))
            .unwrap_or(session.path.clone());

        self.client
            .create_session(&session.name, &path, &session.env, &session.shell)?;
        self.client
            .setenv(&tmux_target!(&session.name), LAIO_CONFIG, config);

        {
            let _guard = self.runtime.enter();
            self.client.flush_commands();
        }

        self.process_windows(session, &dimensions, skip_cmds)?;

        self.client.bind_key(
            "prefix M-l",
            "display-popup -w 50 -h 16 -E 'laio start --show-picker'",
        )?;

        let is_inside_session = self.client.is_inside_session();

        {
            let _guard = self.runtime.enter();
            self.client.flush_commands();
        }

        if !skip_attach {
            if is_inside_session {
                self.client.switch_client(session.name.as_str())?;
                // Now wait for tasks to complete since CLI will exit
                let _guard = self.runtime.enter();
                self.runtime.block_on(self.client.wait_for_tasks())?;
            } else {
                // attach_session blocks, tasks run in background
                self.client.attach_session(session.name.as_str())?;
            }
        } else {
            // CLI exits immediately, must wait for tasks
            let _guard = self.runtime.enter();
            self.runtime.block_on(self.client.wait_for_tasks())?;
        }

        Ok(())
    }

    fn stop(
        &self,
        name: &Option<String>,
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()> {
        let current_session_name = self.client.current_session_name()?;
        log::trace!("Current session name: {current_session_name}");

        if !stop_all && !stop_other && name.is_none() && !self.client.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if (stop_all || stop_other) && name.is_some() {
            bail!("Stopping all/other and specifying a session name are mutually exclusive.")
        };

        if stop_all || (stop_other && self.client.is_inside_session()) {
            log::trace!("Closing all/other laio sessions.");
            self.list_sessions()?
                .into_iter()
                .filter(|info| info.name != current_session_name)
                .try_for_each(|info| -> Result<()> {
                    if self.is_laio_session(&info.name)? {
                        log::trace!("Closing session: {:?}", info.name);
                        self.stop(&Some(info.name.to_string()), skip_cmds, false, false)?;
                    }
                    Ok(())
                })?;
            if !self.client.is_inside_session() {
                log::debug!("Not inside a session");
                return Ok(());
            }
        };

        let name = name.clone().unwrap_or(current_session_name.to_string());
        if !self.client.session_exists(&name) {
            bail!("Session {} does not exist!", &name);
        }
        if !self.is_laio_session(&name)? {
            log::debug!("Not a laio session: {}", &name);
            return Ok(());
        }

        let result = (|| -> Result<()> {
            if !skip_cmds && !stop_other {
                match self.client.getenv(&tmux_target!(&name), LAIO_CONFIG) {
                    Ok(config) => {
                        log::trace!("Config: {config:?}");

                        let session =
                            Session::from_config(&resolve_symlink(&to_absolute_path(&config)?)?)?;

                        let mut commands = session.shutdown.clone();
                        if let Some(script) = &session.shutdown_script {
                            commands.push(script.to_cmd()?);
                        }

                        self.client.run_commands(&commands, &session.path)
                    }
                    Err(e) => {
                        log::warn!("LAIO_CONFIG environment variable not found: {e:?}");
                        Ok(())
                    }
                }
            } else {
                log::trace!("Skipping shutdown commands for session: {name:?}");
                Ok(())
            }
        })();

        let stop_result = if !stop_other {
            self.client.stop_session(name.as_str())
        } else {
            Ok(())
        };

        result.and(stop_result)
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        self.client.list_sessions().map(|sessions| {
            sessions
                .into_iter()
                .map(|(name, is_attached)| SessionInfo::new(name, is_attached))
                .collect()
        })
    }

    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        if self.client.session_exists(name) {
            log::warn!("Session '{name}' already exists");
            if !skip_attach {
                if self.client.is_inside_session() {
                    self.client.switch_client(name)?;
                } else {
                    self.client.attach_session(name)?;
                }
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn get_session(&self) -> Result<Session> {
        let home_dir = home_dir()?;
        let layout = self.client.session_layout()?;
        let name = self.client.session_name()?;
        let path = self.client.session_start_path()?.replace(&home_dir, "~");
        let pane_paths = self.client.pane_paths()?;

        let cmd_dict = self.client.pane_command()?;

        log::trace!("session_layout: {layout}");

        let tokens = parse(&layout, &pane_paths, &path, &cmd_dict);
        log::trace!("tokens: {tokens:#?}");

        Ok(Session::from_tokens(&name, &path, &tokens))
    }
}
