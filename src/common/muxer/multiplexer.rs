use miette::Result;

use crate::common::{config::Session, session_info::SessionInfo};
pub(crate) trait Multiplexer {
    fn start(
        &self,
        session: &Session,
        config: &str,
        skip_attach: bool,
        skip_cmds: bool,
    ) -> Result<()>;
    fn stop(
        &self,
        name: &Option<String>,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()>;
    fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool>;
    fn get_session(&self) -> Result<Session>;
}
