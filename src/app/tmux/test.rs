use std::rc::Rc;

use crate::app::{
    cmd::test::MockRunner,
    tmux::{target::Target, Client},
};

#[test]
fn new_session() -> Result<(), anyhow::Error> {
    let tmux = Client::new(Rc::new(MockRunner::new()));
    let session_name = "test";

    tmux.create_session(&String::from("test"), &String::from("/tmp"))?;
    tmux.new_window(&session_name, &"test".to_string(), &"/tmp".to_string())?;
    tmux.select_layout(
        &Target::new(&session_name).window("@1"),
        &"main-horizontal".to_string(),
    )?;

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
