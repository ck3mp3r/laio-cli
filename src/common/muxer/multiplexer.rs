use anyhow::Result;

use crate::common::config::Session;
pub(crate) trait Multiplexer {
    fn start(
        &self,
        session: &Session,
        config: &str,
        skip_attach: bool,
        skip_cmds: bool,
    ) -> Result<()>;
    fn stop(&self, name: &Option<String>, skip_cmds: bool, stop_all: bool) -> Result<()>;
    fn list_sessions(&self) -> Result<Vec<String>>;
    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool>;
    fn get_session(&self) -> Result<Session>;
}
