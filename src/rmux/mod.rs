pub mod cli;
pub mod config;

use std::env::{current_dir, var};
use std::error::Error;
use std::fs::read_to_string;
use std::io::stdin;
use std::rc::Rc;
use std::{env, fs};

use crate::cmd::CmdRunner;
use crate::tmux::Tmux;

use self::config::Session;

#[derive(Debug)]
pub(crate) struct Rmux<R: CmdRunner> {
    pub config_path: String,
    cmd_runner: Rc<R>,
}

impl<R: CmdRunner> Rmux<R> {
    pub(crate) fn new(config_path: String, cmd_runner: Rc<R>) -> Self {
        Self {
            config_path: config_path.replace("~", env::var("HOME").unwrap().as_str()),
            cmd_runner,
        }
    }

    pub(crate) fn new_config(
        &self,
        name: String,
        copy: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        if copy.is_some() {
            println!("copy: {:?}", copy);
            todo!()
        }
        self.cmd_runner.run(&format!(
            "{} {}/{}.yaml",
            var("EDITOR").unwrap_or_else(|_| "vim".to_string()),
            self.config_path,
            name
        ))
    }

    pub(crate) fn edit_config(&self, name: String) -> Result<(), Box<dyn Error>> {
        self.cmd_runner.run(&format!(
            "{} {}/{}.yaml",
            var("EDITOR").unwrap_or_else(|_| "vim".to_string()),
            self.config_path,
            name
        ))
    }

    pub(crate) fn delete_config(&self, name: String, force: bool) -> Result<(), Box<dyn Error>> {
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

    pub(crate) fn start_session(
        &self,
        name: Option<String>,
        attach: bool,
    ) -> Result<(), Box<dyn Error>> {
        // figure out the config to load
        let config = match name {
            Some(name) => format!("{}/{}.yaml", &self.config_path, name),
            None => {
                let local_config = current_dir()?.join(".rmux").to_string_lossy().to_string();
                format!("{}.yaml", local_config)
            }
        };

        // Read the YAML file into a string
        let config_str = read_to_string(config).unwrap();

        // Parse the YAML into a `Session` struct
        let session: Session = serde_yaml::from_str(&config_str).unwrap();

        let tmux = Tmux::new(Some(session.name), Rc::clone(&self.cmd_runner));

        // check if session already exists
        if tmux.session_exists() {
            println!("Session already exists");
            if attach {
                if tmux.is_inside_session() {
                    tmux.switch_client()?;
                } else {
                    tmux.attach_session()?;
                }
            }
            return Ok(());
        }

        // create the session
        tmux.create_session()?;

        // TODO: run init commands

        // create windows
        for i in 0..session.windows.len() {
            let window = &session.windows[i];

            let idx: i32 = (i + 1).try_into().unwrap();

            let path = match &window.path {
                Some(path) => path,
                None => match &session.path {
                    Some(path) => path,
                    None => ".",
                },
            };

            // create new window
            let window_id = tmux.new_window(&window.name, &path.to_string())?;

            // send commands to window
            tmux.send_keys(format!("{}", window_id), &window.commands)?;

            // select layout
            tmux.select_layout(&window_id, &window.layout)?;

            // delete first window and move others
            if idx == 1 {
                tmux.delete_window(1)?;
                tmux.move_windows()?;
            }

            // create panes
            for n in 0..window.panes.len() {
                let pane = &window.panes[n];

                let pane_id = tmux.split_window(format!("{}", window_id), pane)?;

                // send commands to pane
                tmux.send_keys(format!("{}.{}", window_id, pane_id), &pane.commands)?;

                // delete first pane
                if n == 0 {
                    tmux.delete_pane(idx, 1)?;
                }

                // select layout
                if n % 2 == 0 {
                    tmux.select_layout(&window_id, &"tiled".to_string())?;
                }
            }
        }

        // attach to or switch to session
        if attach {
            if tmux.is_inside_session() {
                tmux.switch_client()?;
            } else {
                tmux.attach_session()?;
            }
        }

        Ok(())
    }

    pub(crate) fn stop_session(&self, name: Option<String>) -> Result<(), Box<dyn Error>> {
        let tmux = Tmux::new(name.clone(), Rc::clone(&self.cmd_runner));
        tmux.stop_session(name)
    }

    pub(crate) fn list_config(&self) -> Result<(), Box<dyn Error>> {
        let mut entries: Vec<String> = Vec::new();

        for entry in fs::read_dir(&self.config_path)? {
            let path = entry?.path();
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if ext == "yaml" {
                    if let Some(file_name) = path.file_stem().and_then(|name| name.to_str()) {
                        entries.push(file_name.to_string());
                    }
                }
            }
        }

        if entries.is_empty() {
            println!("No configurations found.");
        } else {
            println!("Available configurations:");
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
    use super::Rmux;
    use crate::cmd::test::MockCmdRunner;
    use std::env::current_dir;
    use std::env::var;
    use std::rc::Rc;

    #[test]
    fn new_config() {
        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let rmux = Rmux::new("/tmp/rmux".to_string(), Rc::clone(&cmd_runner));

        rmux.new_config(session_name.to_string(), None).unwrap();
        let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        let cmds = rmux.cmd_runner().cmds().borrow();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], format!("{} /tmp/rmux/test.yaml", editor));
    }

    #[test]
    fn edit_config() {
        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let rmux = Rmux::new("/tmp/rmux".to_string(), Rc::clone(&cmd_runner));

        rmux.edit_config(session_name.to_string()).unwrap();
        let editor = var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        let cmds = rmux.cmd_runner().cmds().borrow();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], format!("{} /tmp/rmux/test.yaml", editor));
    }

    #[test]
    fn stop_session() {
        let cwd = current_dir().unwrap();

        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let rmux = Rmux::new(
            format!("{}/src/rmux/test", cwd.to_string_lossy()),
            Rc::clone(&cmd_runner),
        );

        let res = rmux.stop_session(Some(session_name.to_string()));
        let cmds = rmux.cmd_runner().cmds().borrow();
        match res {
            Ok(_) => {
                assert_eq!(cmds.len(), 1);
                assert_eq!(cmds[0], "tmux kill-session -t test")
            }
            Err(e) => assert_eq!(e.to_string(), "Session not found"),
        }
    }

    #[test]
    fn start_session() {
        let cwd = current_dir().unwrap();

        let session_name = "test";
        let cmd_runner = Rc::new(MockCmdRunner::new());
        let rmux = Rmux::new(
            format!("{}/src/rmux/test", cwd.to_string_lossy()),
            Rc::clone(&cmd_runner),
        );

        let res = rmux.start_session(Some(session_name.to_string()), true);
        let mut cmds = rmux.cmd_runner().cmds().borrow().clone();
        match res {
            Ok(_) => {
                // assert_eq!(cmds.len(), 1);
                assert_eq!(cmds.remove(0).to_string(), "tmux has-session -t test");
                assert_eq!(cmds.remove(0).to_string(), "tmux new-session -d -s test");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux new-window -Pd -t test -n code -c /tmp -F \"#{window_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1 'echo \"hello world\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 main-vertical"
                );
                assert_eq!(cmds.remove(0).to_string(), "tmux kill-window -t test:1");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux move-window -r -s test -t test"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -Pd -t test:@1 -h -c . -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1.%1 'echo \"hello\"' C-m"
                );
                assert_eq!(cmds.remove(0).to_string(), "tmux kill-pane -t test:1.1");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@1 tiled"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -Pd -t test:@1 -v -c src -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@1.%2 'echo \"hello again\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux new-window -Pd -t test -n infrastructure -c . -F \"#{window_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 tiled"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -Pd -t test:@2 -h -c one -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%3 'echo \"hello again 1\"' C-m"
                );
                assert_eq!(cmds.remove(0).to_string(), "tmux kill-pane -t test:2.1");
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 tiled"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -Pd -t test:@2 -h -c two -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%4 'echo \"hello again 2\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux split-window -Pd -t test:@2 -h -c three -F \"#{pane_id}\""
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%3 'clear' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux send-keys -t test:@2.%3 'echo \"hello again 3\"' C-m"
                );
                assert_eq!(
                    cmds.remove(0).to_string(),
                    "tmux select-layout -t test:@2 tiled"
                );
                assert_eq!(cmds.remove(0).to_string(), "printenv TMUX");
                assert_eq!(cmds.remove(0).to_string(), "tmux attach-session -t test:1");
            }
            Err(e) => assert_eq!(e.to_string(), "Session not found"),
        }
    }
}
