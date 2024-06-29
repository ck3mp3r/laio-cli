use std::rc::Rc;

use crate::app::{cmd_test::test::MockCmdRunner, tmux_client::TmuxClient};

#[test]
fn new_session() -> Result<(), anyhow::Error> {
    let mock_cmd_runner = Rc::new(MockCmdRunner::new());
    let tmux = TmuxClient::new(
        &Some(String::from("test")),
        &String::from("/tmp"),
        Rc::clone(&mock_cmd_runner),
    );

    tmux.create_session(&".laio.yaml".to_string())?;
    tmux.new_window(&"test".to_string(), &"/tmp".to_string())?;
    tmux.select_layout(&"@1".to_string(), &"main-horizontal".to_string())?;

    let mut cmds = tmux.cmd_runner.cmds().borrow_mut();
    assert_eq!(
        cmds.remove(0).to_string(),
        "tmux new-session -d -s \"test\" -c \"/tmp\""
    );
    assert_eq!(
        cmds.remove(0).to_string(),
        "tmux new-window -Pd -t \"test\" -n \"test\" -c \"/tmp\" -F \"#{window_id}\""
    );
    assert_eq!(
        cmds.remove(0).to_string(),
        "tmux select-layout -t \"test\":@1 \"main-horizontal\""
    );
    Ok(())
}
