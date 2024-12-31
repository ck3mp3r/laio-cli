use std::{
    env::temp_dir,
    fs::{remove_file, File},
    rc::Rc,
};

use crate::common::{mux::client::Client, path::sanitize_filename};
use crate::{
    cmd_basic, cmd_forget,
    common::cmd::{Runner, Type},
};
use anyhow::Result;
use kdl::{KdlDocument, KdlNode};

#[derive(Debug)]
pub(crate) struct ZellijClient<R: Runner> {
    pub cmd_runner: Rc<R>,
}

impl<R: Runner> Client<R> for ZellijClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> ZellijClient<R> {
    pub(crate) fn new(cmd_runner: Rc<R>) -> Self {
        Self { cmd_runner }
    }

    pub(crate) fn create_session_with_layout(
        &self,
        name: &str,
        config: &str,
        layout: &str,
        skip_attach: bool,
    ) -> Result<()> {
        let cmd = if skip_attach {
            // workaround as zellij doesn't yet support backgrounding when creating with a layout.
            &cmd_forget!(
                "LAIO_CONFIG={} nohup zellij --session {} --new-session-with-layout {} > /dev/null 2>&1 </dev/null & disown",
              config,
              name,
              layout
            )
        } else {
            &cmd_forget!(
                "LAIO_CONFIG={} zellij --session {} --new-session-with-layout {}",
                config,
                name,
                layout
            )
        };
        self.cmd_runner.run(cmd)
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<()> {
        self.session_exists(name)
            .then(|| {
                self.cmd_runner
                    .run(&cmd_basic!("zellij delete-session \"{}\" --force", name))
            })
            .unwrap_or(Ok(()))
    }

    pub(crate) fn attach(&self, name: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_forget!("zellij attach {} ", name))
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!(
                "zellij list-sessions --short | grep \"{}\"",
                name
            ))
            .unwrap_or(false)
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv ZELLIJ"))
            .is_ok_and(|s: String| !s.is_empty())
    }

    pub(crate) fn current_session_name(&self) -> Result<String> {
        self.cmd_runner
            .run(&cmd_basic!("printenv ZELLIJ_SESSION_NAME || true"))
    }

    pub(crate) fn getenv(&self, name: &str, key: &str) -> Result<String> {
        if self.is_inside_session() {
            self.cmd_runner.run(&cmd_basic!("printenv {} || true", key))
        } else {
            // workaround as zellij does not really support scripting with output to stdout
            let mut temp_path = temp_dir();
            temp_path.push(format!("{}.tmp", sanitize_filename(name)));
            let temp_path_str = temp_path.to_str().unwrap().to_string();
            let _temp_file = File::create(&temp_path)?;

            let _res: () = self.cmd_runner.run(&cmd_basic!(
                "zellij run -c --name {} -- sh -c \"printenv {} > {}\"",
                name,
                key,
                temp_path_str
            ))?;
            let result = self.cmd_runner.run(&cmd_basic!("cat {}", temp_path_str))?;
            remove_file(temp_path_str)?;

            Ok(result)
        }
    }

    pub(crate) fn list_sessions(&self) -> Result<Vec<String>> {
        self.cmd_runner
            .run(&cmd_basic!("zellij list-sessions --short"))
            .map(|res: String| res.lines().map(String::from).collect())
            .or_else(|_| Ok(vec![]))
    }

    pub(crate) fn get_layout(&self) -> Result<KdlNode> {
        let res: String = self
            .cmd_runner
            .run(&cmd_basic!("zellij action dump-layout"))?;
        let kdl_doc = KdlDocument::parse_v1(res.as_str())?;
        let layout_node = kdl_doc
            .nodes()
            .first()
            .cloned()
            .expect("Missing layout node.");
        Ok(layout_node)
    }
}
