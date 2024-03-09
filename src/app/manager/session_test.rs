use crate::{
    app::{cmd_test::test::MockCmdRunner, manager::session::SessionManager},
    util::path::current_working_path,
};

use std::sync::Once;
use std::{env::current_dir, rc::Rc};

static INIT: Once = Once::new();

pub fn initialize() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

#[test]
fn session_stop() {
    initialize();
    let cwd = current_working_path().expect("Cannot get current working directory");

    let session_name = "foo";
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let session = SessionManager::new(
        &format!("{}/src/app/manager/test", cwd.to_string_lossy()),
        Rc::clone(&cmd_runner),
    );

    let res = session.stop(&Some(session_name.to_string()), &false);
    let cmds = session.cmd_runner().cmds().borrow();
    match res {
        Ok(_) => {
            assert_eq!(cmds.len(), 6);
            assert_eq!(cmds[0].as_str(), "tmux has-session -t \"foo\"");
            assert_eq!(
                cmds[1].as_str(),
                "tmux show-environment -t \"foo\": LAIO_CONFIG"
            );
            assert_eq!(cmds[2].as_str(), "dates");
            assert_eq!(cmds[3].as_str(), "echo Bye");
            assert_eq!(cmds[4].as_str(), "tmux has-session -t \"foo\"");
            assert_eq!(cmds[5].as_str(), "tmux kill-session -t \"foo\"");
        }
        Err(e) => assert_eq!(
            e.to_string(),
            format!("Session {} does not exist!", session_name)
        ),
    }
}

#[test]
fn session_list() {
    initialize();
    let cwd = current_working_path().expect("Cannot get current working directory");

    let cmd_runner = Rc::new(MockCmdRunner::new());
    let session = SessionManager::new(
        &format!("{}/src/app/manager/test", cwd.to_string_lossy()),
        Rc::clone(&cmd_runner),
    );

    let res = session.list();
    let cmds = session.cmd_runner().cmds().borrow();
    println!("{:?}", cmds);
    match res {
        Ok(_) => {
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].as_str(), "tmux display-message -p \\#S");
            assert_eq!(cmds[1].as_str(), "tmux ls -F \"#{session_name}\"");
        }
        Err(e) => assert_eq!(e.to_string(), "No active sessions."),
    }
}

#[test]
fn session_start() {
    initialize();
    let cwd = current_working_path().expect("Cannot get current working directory");

    let session_name = "valid";
    let cmd_runner = Rc::new(MockCmdRunner::new());
    let session = SessionManager::new(
        &format!("{}/src/app/manager/test", cwd.to_string_lossy()),
        Rc::clone(&cmd_runner),
    );

    let res = session.start(
        &Some(session_name.to_string()),
        &".foo.yaml".to_string(),
        &false,
    );
    let mut cmds = session.cmd_runner().cmds().borrow().clone();
    println!("{:?}", cmds);
    match res {
        Ok(_) => {
            assert_eq!(cmds.remove(0).to_string(), "tmux has-session -t \"valid\"");
            assert_eq!(cmds.remove(0).to_string(), "tmux has-session -t \"valid\"");
            assert_eq!(cmds.remove(0).to_string(), "printenv TMUX");
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux display-message -p \"width: #{window_width}\nheight: #{window_height}\""
            );
            assert_eq!(cmds.remove(0).to_string(), "date");
            assert_eq!(cmds.remove(0).to_string(), "echo Hi");
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux new-session -d -s \"valid\" -c /tmp"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux show-options -g base-index"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux new-window -Pd -t \"valid\" -n \"code\" -c /tmp -F \"#{window_id}\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux kill-window -t \"valid\":1"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux move-window -r -s \"valid\" -t \"valid\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux display-message -t \"valid\":@1 -p \"#P\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@1 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux display-message -t \"valid\":@1 -p \"#P\""
            );
            // // assert_eq!(cmds.remove(0).to_string(), "tmux kill-pane -t test:1.1");
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-pane -t \"valid\":@1.%2 -P 'bg=red,fg=default'"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@1 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux split-window -t \"valid\":@1 -c /tmp -P -F \"#{pane_id}\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@1 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux split-window -t \"valid\":@1 -c /tmp/src -P -F \"#{pane_id}\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@1 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@1 \"83ed,160x90,0,0[160x45,0,0{53x45,0,0,2,106x45,54,0,3},160x44,0,46,4]\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux new-window -Pd -t \"valid\" -n \"infrastructure\" -c /tmp -F \"#{window_id}\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux display-message -t \"valid\":@2 -p \"#P\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@2 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux split-window -t \"valid\":@2 -c /tmp/two -P -F \"#{pane_id}\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@2 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux split-window -t \"valid\":@2 -c /tmp/three -P -F \"#{pane_id}\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@2 \"tiled\""
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux select-layout -t \"valid\":@2 \"149e,160x90,0,0[160x22,0,0,5,160x45,0,23,6,160x21,0,69,7]\""
            );
            assert!(cmds
                .remove(0)
                .to_string()
                .starts_with("tmux bind-key -T"));
            assert!(cmds
                .remove(0)
                .to_string()
                .starts_with("tmux setenv -t \"valid\": LAIO_CONFIG"));
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@1.%1 'cd /tmp' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@1.%2 'cd /tmp' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@1.%1 'echo \"hello\"' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@1.%4 'echo \"hello again\"' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@2.%5 'cd /tmp/one' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@2.%5 'echo \"hello again 1\"' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@2.%6 'echo \"hello again 2\"' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@2.%7 'clear' C-m"
            );
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux send-keys -t \"valid\":@2.%7 'echo \"hello again 3\"' C-m"
            );
            assert_eq!(cmds.remove(0).to_string(), "printenv TMUX");
            assert_eq!(
                cmds.remove(0).to_string(),
                "tmux switch-client -t \"valid\""
            );
        }
        Err(e) => assert_eq!(e.to_string(), "Session not found"),
    }
}

#[test]
fn session_to_yaml() {
    initialize();
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
