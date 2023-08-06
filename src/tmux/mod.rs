use serde::Deserialize;

use crate::cmd::CmdRunner;
use std::{error::Error, fmt::Debug, rc::Rc};

#[derive(Debug, Deserialize)]
pub(crate) struct Dimensions {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub(crate) struct Tmux<R: CmdRunner> {
    pub session_name: String,
    pub session_path: String,
    pub dimensions: Dimensions,
    pub cmd_runner: Rc<R>,
}

impl<R: CmdRunner> Tmux<R> {
    pub(crate) fn new(
        session_name: &Option<String>,
        session_path: &Option<String>,
        cmd_runner: Rc<R>,
    ) -> Self {
        Self {
            session_name: match session_name {
                Some(s) => s.clone(),
                None => cmd_runner
                    .run(&format!("tmux display-message -p \\#S"))
                    .unwrap_or_else(|_| "rmux".to_string()),
            },
            session_path: match session_path {
                Some(s) => s.clone(),
                None => cmd_runner
                    .run(&format!(
                        "tmux display-message -p \"#{{session_base_path}}\""
                    ))
                    .unwrap_or_else(|_| ".".to_string()),
            },
            dimensions: {
                let res = cmd_runner.run(
                    &format!("tmux display-message -p \"width: #{{window_width}}\nheight: #{{window_height}}\""))
                    .unwrap_or_else(|_| "width: 160\nheight: 90".to_string());
                let dims: Dimensions = serde_yaml::from_str(&res).expect("doh!");
                dbg!(&dims);
                dims
            },
            cmd_runner,
        }
    }

    pub(crate) fn create_session(&self) -> Result<(), Box<dyn Error>> {
        self.cmd_runner.run(&format!(
            "tmux new-session -d -s {} -c {}",
            self.session_name, self.session_path
        ))
    }

    pub(crate) fn session_exists(&self) -> bool {
        let v: Result<bool, Box<dyn Error>> = self
            .cmd_runner
            .run(&format!("tmux has-session -t {}", self.session_name));
        match v {
            Ok(s) => s,
            Err(_) => false,
        }
    }

    pub(crate) fn switch_client(&self) -> Result<(), Box<dyn Error>> {
        self.cmd_runner
            .run(&format!("tmux switch-client -t {}:1", self.session_name))
    }

    pub(crate) fn attach_session(&self) -> Result<(), Box<dyn Error>> {
        self.cmd_runner
            .run(&format!("tmux attach-session -t {}:1", self.session_name))
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        let v: Result<String, Box<dyn Error>> = self.cmd_runner.run(&format!("printenv TMUX"));
        match v {
            Ok(s) => s != "",
            Err(_) => false,
        }
    }

    pub(crate) fn stop_session(&self, name: &Option<String>) -> Result<(), Box<dyn Error>> {
        let session_name = match name {
            Some(s) => s.clone(),
            None => self.session_name.clone(),
        };
        self.cmd_runner
            .run(&format!("tmux kill-session -t {}", session_name))
    }

    // pub(crate) fn get_session_name(&self) -> Option<String> {
    //     self.cmd_runner
    //         .run(&format!("tmux display-message -p #S"))
    //         .ok()
    // }

    pub(crate) fn new_window(
        &self,
        window_name: &String,
        path: &String,
    ) -> Result<String, Box<dyn Error>> {
        self.cmd_runner.run(&format!(
            "tmux new-window -Pd -t {} -n {} -c {} -F \"#{{window_id}}\"",
            &self.session_name, window_name, path
        ))
    }

    // pub(crate) fn rename_window(&self, pos: i32, window_name: &str) -> Result<(), Box<dyn Error>> {
    //     self.cmd_runner.run(&format!(
    //         "tmux rename-window -t {}:{} {}",
    //         &self.session_name, pos, window_name
    //     ))
    // }

    pub(crate) fn delete_window(&self, pos: i32) -> Result<(), Box<dyn Error>> {
        self.cmd_runner.run(&format!(
            "tmux kill-window -t {}:{}",
            &self.session_name, pos
        ))
    }

    pub(crate) fn move_windows(&self) -> Result<(), Box<dyn Error>> {
        self.cmd_runner.run(&format!(
            "tmux move-window -r -s {} -t {}",
            &self.session_name, &self.session_name
        ))
    }

    //cmd := exec.Command("tmux", "move-window", "-r", "-s", target, "-t", target)

    pub(crate) fn split_window(
        &self,
        target: &String,
        orientation: &String,
        size: &u8,
        path: &String,
    ) -> Result<String, Box<dyn Error>> {
        // dbg!("split_window: {:?}", pane);

        self.cmd_runner.run(&format!(
            "tmux split-window -t {}:{} -{} -p {} -c {} -F \"#{{pane_id}}\"",
            &self.session_name, target, orientation, size, path
        ))
    }

    // pub(crate) fn get_current_pane(&self, target: String) -> Result<String, Box<dyn Error>> {
    //     self.cmd_runner.run(&format!(
    //         "tmux display-message -t {}:{} -p \"#P\"",
    //         &self.session_name, target
    //     ))
    // }

    pub(crate) fn send_keys(
        &self,
        target: &String,
        cmds: &Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        for cmd in cmds {
            self.cmd_runner.run(&format!(
                "tmux send-keys -t {}:{} '{}' C-m",
                &self.session_name, target, cmd
            ))?;
        }

        Ok(())
    }

    // pub(crate) fn delete_pane(&self, idx: i32, pos: i32) -> Result<(), Box<dyn Error>> {
    //     self.cmd_runner.run(&format!(
    //         "tmux kill-pane -t {}:{}.{}",
    //         &self.session_name, idx, pos
    //     ))
    // }

    pub(crate) fn select_layout(
        &self,
        target: &String,
        layout: &String,
    ) -> Result<(), Box<dyn Error>> {
        self.cmd_runner.run(&format!(
            "tmux select-layout -t {}:{} {}",
            &self.session_name, target, layout
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::cmd::test::MockCmdRunner;
    use crate::tmux::Tmux;
    use std::error::Error;
    use std::rc::Rc;

    #[test]
    fn new_session() -> Result<(), Box<dyn Error>> {
        let mock_cmd_runner = Rc::new(MockCmdRunner::new());
        let tmux = Tmux::new(
            &Some(String::from("test")),
            &Some(String::from("/tmp")),
            Rc::clone(&mock_cmd_runner),
        );

        tmux.create_session()?;
        tmux.new_window(&"test".to_string(), &"/tmp".to_string())?;
        tmux.select_layout(&"@1".to_string(), &"main-horizontal".to_string())?;

        let cmds = tmux.cmd_runner.get_cmds();
        assert_eq!(
            cmds[0],
            "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\""
        );
        assert_eq!(cmds[1], "tmux new-session -d -s test -c /tmp");
        assert_eq!(
            cmds[2],
            "tmux new-window -Pd -t test -n test -c /tmp -F \"#{window_id}\""
        );
        assert_eq!(cmds[3], "tmux select-layout -t test:@1 main-horizontal");
        Ok(())
    }
}
