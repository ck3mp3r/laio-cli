use std::{env, fs::read_to_string, rc::Rc};

use anyhow::Error;

use crate::app::{
    cmd::CmdRunner,
    config::{FlexDirection, Pane, Session},
    parser::parse,
    tmux::{Dimensions, Tmux},
};

#[derive(Debug)]
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
        attach: &bool,
    ) -> Result<(), Error> {
        let config = match name {
            Some(name) => format!("{}/{}.yaml", &self.config_path, name),
            None => file.to_string(),
        };

        log::info!("Loading config: {}", config);

        let config_str = read_to_string(config)?;

        let session: Session = serde_yaml::from_str(&config_str)?;

        // create tmux client
        let tmux = Tmux::new(
            &Some(session.name.clone()),
            &session.path.to_owned(),
            Rc::clone(&self.cmd_runner),
        );

        // check if session already exists
        if tmux.session_exists(session.name.as_str()) {
            log::warn!("Session '{}' already exists", &session.name);
            if *attach {
                if tmux.is_inside_session() {
                    tmux.switch_client()?;
                } else {
                    tmux.attach_session()?;
                }
            }
            return Ok(());
        }

        let dimensions = tmux.get_dimensions()?;

        self.run_init_commands(&session)?;

        tmux.create_session()?;

        self.initialise_windows(&session, &tmux, &dimensions)?;

        tmux.flush_commands()?;

        if *attach && tmux.is_inside_session() {
            tmux.switch_client()?;
        } else if !tmux.is_inside_session() {
            tmux.attach_session()?;
        }

        Ok(())
    }

    pub(crate) fn stop(&self, name: &Option<String>) -> Result<(), Error> {
        let session_name = name.as_ref().map_or("", String::as_str);
        Tmux::new(name, &None, Rc::clone(&self.cmd_runner)).stop_session(session_name)
    }

    pub(crate) fn list(&self) -> Result<(), Error> {
        let sessions = Tmux::new(&None, &None, Rc::clone(&self.cmd_runner)).list_sessions()?;

        if sessions.is_empty() {
            println!("No active sessions found.");
        } else {
            println!("Active Sessions:");
            println!("----------------");
            println!("{}", sessions.join("\n"));
        }
        Ok(())
    }

    pub(crate) fn to_yaml(&self) -> Result<(), Error> {
        let res: String = self.cmd_runner.run(&format!(
            "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\""
        ))?;
        let name: String = self
            .cmd_runner
            .run(&format!("tmux display-message -p \"#S\""))?;

        log::debug!("session_to_yaml: {}", res);

        let tokens = parse(&res);
        log::debug!("tokens: {:#?}", tokens);

        let session = Session::from_tokens(&name, &tokens);
        log::debug!("session: {:#?}", session);

        let yaml = serde_yaml::to_string(&session)?;

        println!("{}", yaml);

        Ok(())
    }

    fn initialise_windows(
        &self,
        session: &Session,
        tmux: &Tmux<R>,
        dimensions: &Dimensions,
    ) -> Result<(), Error> {
        Ok(for i in 0..session.windows.len() {
            let window = &session.windows[i];

            let base_idx = tmux.get_base_idx()?;
            log::trace!("base-index: {}", base_idx);

            let idx = i + base_idx;

            let window_path =
                self.sanitize_path(&window.path, &session.path.to_owned().unwrap().clone());

            // create new window
            let window_id = tmux.new_window(&window.name, &window_path.to_string())?;
            log::trace!("window-id: {}", window_id);

            // register commands
            tmux.register_commands(&window_id, &window.commands);

            // delete first window and move others
            if idx == base_idx {
                tmux.delete_window(base_idx)?;
                tmux.move_windows()?;
            }

            // create layout string
            let layout = self.generate_layout_string(
                &window_id,
                &window_path,
                &window.panes,
                dimensions.width,
                dimensions.height,
                &window.flex_direction,
                0,
                0,
                tmux,
                0,
            )?;

            log::trace!("layout: {}", layout);

            // apply layout to window
            tmux.select_layout(
                &window_id,
                &format!("{},{}", tmux.layout_checksum(&layout), layout),
            )?;
        })
    }

    fn run_init_commands(&self, session: &Session) -> Result<(), Error> {
        Ok(if session.commands.len() > 0 {
            log::info!("Running init commands...");
            for c in 0..session.commands.len() {
                let cmd = &session.commands[c];
                let res: String = self.cmd_runner.run(cmd)?;
                log::info!("\n{}\n{}", cmd, res);
            }
            log::info!("Completed init commands.");
        })
    }

    fn sanitize_path(&self, path: &Option<String>, window_path: &String) -> String {
        match path {
            Some(path) if path.starts_with("/") || path.starts_with("~") => path.clone(),
            Some(path) if path == "." => window_path.clone(),
            Some(path) => format!(
                "{}/{}",
                window_path,
                path.strip_prefix("./").unwrap_or(path)
            ),
            None => window_path.clone(),
        }
    }

    fn generate_layout_string(
        &self,
        window_id: &str,
        window_path: &str,
        panes: &[Pane],
        width: usize,
        height: usize,
        direction: &Option<FlexDirection>,
        start_x: usize,
        start_y: usize,
        tmux: &Tmux<R>,
        depth: usize,
    ) -> Result<String, Error> {
        let total_flex = panes.iter().map(|p| p.flex.unwrap_or(1)).sum::<usize>();

        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut pane_strings: Vec<String> = Vec::new();

        let mut dividers = 0;

        for (index, pane) in panes.iter().enumerate() {
            let flex = pane.flex.unwrap_or(1);

            let (pane_width, pane_height, next_x, next_y) = match direction {
                Some(FlexDirection::Column) => {
                    let w = if index == panes.len() - 1 {
                        log::trace!("width: {}, current_x: {}", width, current_x);
                        if current_x >= width {
                            log::warn!("skipping pane: width: {}, current_x: {}", width, current_x);
                            continue;
                        }
                        width - current_x // give the remaining width to the last pane
                    } else if depth > 0 || index > 0 {
                        width * flex / total_flex - dividers
                    } else {
                        width * flex / total_flex
                    };
                    (w, height, current_x + w + 1, current_y)
                }
                _ => {
                    let h = if index == panes.len() - 1 {
                        log::trace!("height: {}, current_y: {}", height, current_y);
                        if current_y >= height {
                            log::warn!(
                                "skipping pane: height: {}, current_y: {}",
                                height,
                                current_y
                            );
                            continue;
                        }
                        height - current_y // give the remaining height to the last pane
                    } else if depth > 0 || index > 0 {
                        height * flex / total_flex - dividers
                    } else {
                        height * flex / total_flex
                    };
                    (width, h, current_x, current_y + h + 1)
                }
            };

            // Increment divider count after calculating position and dimension for this pane
            if depth > 0 || index > 0 {
                dividers += 1;
            }

            let path = self.sanitize_path(&pane.path, &window_path.to_string());

            // Create panes in tmux as we go
            let pane_id = if index > 0 {
                tmux.split_window(window_id, &path)?
            } else {
                tmux.get_current_pane(window_id)?
            };

            if index == 0 {
                tmux.register_commands(
                    &format!("{}.{}", window_id, pane_id),
                    &vec![format!("cd {}", &path)],
                );
            };

            tmux.select_layout(window_id, &"tiled".to_string())?;

            if let Some(sub_panes) = &pane.panes {
                pane_strings.push(self.generate_layout_string(
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
                )?);
            } else {
                pane_strings.push(format!(
                    "{0}x{1},{2},{3},{4}",
                    pane_width,
                    pane_height,
                    current_x,
                    current_y,
                    pane_id.replace("%", "")
                ));
            }

            current_x = next_x;
            current_y = next_y;
            tmux.register_commands(&format!("{}.{}", window_id, pane_id), &pane.commands);
        }

        if pane_strings.len() > 1 {
            match direction {
                Some(FlexDirection::Column) => Ok(format!(
                    "{}x{},0,0{{{}}}",
                    width,
                    height,
                    pane_strings.join(",")
                )),
                _ => Ok(format!(
                    "{}x{},0,0[{}]",
                    width,
                    height,
                    pane_strings.join(",")
                )),
            }
        } else {
            Ok(format!("{}x{},0,0", width, height))
        }
    }

    #[cfg(test)]
    pub(crate) fn cmd_runner(&self) -> &R {
        &self.cmd_runner
    }
}

#[cfg(test)]
mod test {
    use crate::app::cmd::test::MockCmdRunner;
    use crate::app::manager::session::SessionManager;
    use std::{env::current_dir, rc::Rc};

    #[test]
    fn session_stop() {
        let cwd = current_dir().unwrap();

        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let session = SessionManager::new(
            &format!("{}/src/session/test", cwd.to_string_lossy()),
            Rc::clone(&cmd_runner),
        );

        let res = session.stop(&Some(session_name.to_string()));
        let cmds = session.cmd_runner().cmds().borrow();
        println!("{:?}", cmds);
        match res {
            Ok(_) => {
                assert_eq!(cmds.len(), 2);
                assert_eq!(cmds[0], "tmux display-message -p \"#{session_base_path}\"");
                assert_eq!(cmds[1], "tmux has-session -t test");
            }
            Err(e) => assert_eq!(e.to_string(), "Session not found"),
        }
    }

    #[test]
    fn session_list() {
        let cwd = current_dir().unwrap();

        let cmd_runner = Rc::new(MockCmdRunner::new());
        let session = SessionManager::new(
            &format!("{}/src/session/test", cwd.to_string_lossy()),
            Rc::clone(&cmd_runner),
        );

        let res = session.list();
        let cmds = session.cmd_runner().cmds().borrow();
        println!("{:?}", cmds);
        match res {
            Ok(_) => {
                assert_eq!(cmds.len(), 3);
                assert_eq!(cmds[0], "tmux display-message -p \\#S");
                assert_eq!(cmds[1], "tmux display-message -p \"#{session_base_path}\"");
                assert_eq!(cmds[2], "tmux ls -F \"#{session_name}\"");
            }
            Err(e) => assert_eq!(e.to_string(), "No active sessions."),
        }
    }

    #[test]
    fn session_start() {
        let cwd = current_dir().unwrap();

        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let session = SessionManager::new(
            &format!("{}/src/commands/session/test", cwd.to_string_lossy()),
            Rc::clone(&cmd_runner),
        );

        let res = session.start(
            &Some(session_name.to_string()),
            &".foo.yaml".to_string(),
            &true,
        );
        let mut cmds = session.cmd_runner().cmds().borrow().clone();
        match res {
            Ok(_) => {
                assert_eq!(cmds.remove(0).to_string(), "tmux has-session -t test");
                assert_eq!(cmds.remove(0).to_string(), "printenv TMUX");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\""
                );
                assert_eq!(cmds.remove(0).to_string(), "date");
                assert_eq!(cmds.remove(0).to_string(), "echo Hi");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux new-session -d -s test -c /tmp"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux show-options -g base-index"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux new-window -Pd -t test -n code -c /tmp -F \"#{window_id}\""
                );
                assert_eq!(cmds.remove(0).to_string(), "tmux kill-window -t test:1");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux move-window -r -s test -t test"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux display-message -t test:@1 -p \"#P\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux display-message -t test:@1 -p \"#P\""
                );
                // // assert_eq!(cmds.remove(0).to_string(), "tmux kill-pane -t test:1.1");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -t test:@1 -c /tmp -P -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -t test:@1 -c /tmp/src -P -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 \"9b85,160x90,0,0{80x90,0,0[80x30,0,0,2,80x59,0,31,3],79x90,81,0,4}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux show-options -g base-index"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux new-window -Pd -t test -n infrastructure -c /tmp -F \"#{window_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux display-message -t test:@2 -p \"#P\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -t test:@2 -c /tmp/two -P -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -t test:@2 -c /tmp/three -P -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 \"tiled\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 \"c301,160x90,0,0{40x90,0,0,5,80x90,41,0,6,38x90,122,0,7}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1 'echo \"hello world\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1.%1 'cd /tmp' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1.%2 'cd /tmp' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1.%1 'echo \"hello\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1.%4 'echo \"hello again\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%5 'cd /tmp/one' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%5 'echo \"hello again 1\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%6 'echo \"hello again 2\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%7 'clear' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%7 'echo \"hello again 3\"' C-m"
                );
                assert_eq!(cmds.remove(0).to_string(), "printenv TMUX");
                assert_eq!(cmds.remove(0).to_string(), "tmux switch-client -t test");
            }
            Err(e) => assert_eq!(e.to_string(), "Session not found"),
        }
    }

    #[test]
    fn session_to_yaml() {
        let cwd = current_dir().unwrap();

        let cmd_runner = Rc::new(MockCmdRunner::new());
        let session = SessionManager::new(
            &format!("{}/src/session/test", cwd.to_string_lossy()),
            Rc::clone(&cmd_runner),
        );

        let _res = session.to_yaml();
        let mut cmds = session.cmd_runner().cmds().borrow().clone();
        assert_eq!(
            cmds.remove(0).to_string(),
            "tmux list-windows -F \"#{window_name} #{window_layout}\""
        );
    }
}
