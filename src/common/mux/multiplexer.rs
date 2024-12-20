use anyhow::Result;

use crate::common::config::Session;
pub(crate) trait Multiplexer {
    fn start(
        &self,
        session: &Session,
        config: &str,
        skip_attach: bool,
        skip_startup_commands: bool,
    ) -> Result<()>;
    fn stop(&self, name: &Option<String>, skip_shutdown_cmds: bool, stop_all: bool) -> Result<()>;
    fn list_sessions(&self) -> Result<Vec<String>>;
    fn try_switch(&self, name: &str, skip_attach: bool) -> Result<bool>;
    fn get_session(&self) -> Result<Session>;
}
