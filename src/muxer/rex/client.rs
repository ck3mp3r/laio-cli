use std::sync::Arc;

use miette::{Report, Result, miette};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

use crate::cmd_basic;
use crate::common::cmd::{Runner, Type};
use crate::common::muxer::Client;

use super::model::{
    CreateOutput, CreateRequest, FocusOutput, PwdOutput, SessionListOutput, SessionView,
};

#[derive(Debug)]
pub(crate) struct RexClient<R: Runner> {
    cmd_runner: Arc<R>,
}

impl<R: Runner> Client<R> for RexClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> RexClient<R> {
    pub(crate) fn new(cmd_runner: Arc<R>) -> Self {
        Self { cmd_runner }
    }

    pub(crate) fn session_create(&self, request: &CreateRequest) -> Result<CreateOutput> {
        let payload = to_json(request)?;

        self.run_json(
            "session_create",
            cmd_basic!("rex", args = ["api", "call", "session.create", payload]),
        )
    }

    pub(crate) fn session_view(&self, session_id: &str) -> Result<SessionView> {
        let payload = to_json(&json!({"session_id": session_id}))?;

        self.run_json(
            "session_view",
            cmd_basic!("rex", args = ["api", "call", "session.view", payload]),
        )
    }

    pub(crate) fn session_focus_window(
        &self,
        session_id: &str,
        window_id: &str,
    ) -> Result<FocusOutput> {
        let payload = to_json(&json!({"session_id": session_id, "window_id": window_id}))?;

        self.run_json(
            "session_focus_window",
            cmd_basic!(
                "rex",
                args = ["api", "call", "session.focus_window", payload]
            ),
        )
    }

    pub(crate) fn session_focus_block(
        &self,
        session_id: &str,
        block_id: &str,
    ) -> Result<FocusOutput> {
        let payload = to_json(&json!({"session_id": session_id, "block_id": block_id}))?;

        self.run_json(
            "session_focus_block",
            cmd_basic!(
                "rex",
                args = ["api", "call", "session.focus_block", payload]
            ),
        )
    }

    pub(crate) fn session_list(&self) -> Result<SessionListOutput> {
        self.run_json(
            "session_list",
            cmd_basic!("rex", args = ["session", "ls", "--json"]),
        )
    }

    pub(crate) fn block_pwd(&self, session: &str, block_id: &str) -> Result<PwdOutput> {
        self.run_json(
            "block_pwd",
            cmd_basic!(
                "rex",
                args = ["block", "call", "-s", session, "-b", block_id, "pwd"]
            ),
        )
    }

    pub(crate) fn zoom(&self, session: &str, block_id: &str, on: bool) -> Result<()> {
        let state = if on { "--on" } else { "--off" };

        self.run_unit(
            "zoom",
            cmd_basic!("rex", args = ["zoom", "-s", session, block_id, state]),
        )
    }

    pub(crate) fn send_commands(&self, session: &str, block_id: &str, text: &str) -> Result<()> {
        self.run_unit(
            "send_commands",
            cmd_basic!(
                "rex",
                args = ["send", "-s", session, "-b", block_id, "--", text]
            ),
        )
    }

    fn run_json<T: DeserializeOwned>(&self, method: &str, cmd: Type) -> Result<T> {
        let output = self.run_string(method, &cmd)?;

        parse_json(method, &output)
    }

    fn run_string(&self, method: &str, cmd: &Type) -> Result<String> {
        let output: String = self
            .cmd_runner
            .run(cmd)
            .map_err(|error| rex_error(method, &error))?;

        Ok(output)
    }

    fn run_unit(&self, method: &str, cmd: Type) -> Result<()> {
        let _: () = self
            .cmd_runner
            .run(&cmd)
            .map_err(|error| rex_error(method, &error))?;

        Ok(())
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| miette!("rex request serialisation failed: {error}"))
}

fn parse_json<T: DeserializeOwned>(method: &str, output: &str) -> Result<T> {
    serde_json::from_str(output)
        .map_err(|error| miette!("rex {method} returned invalid JSON: {error}: {output}"))
}

fn rex_error(method: &str, error: &Report) -> Report {
    let detail = error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(": ");

    miette!("rex {method} failed: {detail}")
}
