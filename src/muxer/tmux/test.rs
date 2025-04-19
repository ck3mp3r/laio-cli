use crate::{
    common::cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
    tmux_target,
};
use crate::{
    common::{config::Session, muxer::multiplexer::Multiplexer},
    muxer::{tmux::Target, Tmux},
};
use lazy_static::lazy_static;
use miette::{IntoDiagnostic, Result};
use serde_yaml::Value;
use std::{
    collections::HashMap,
    env::current_dir,
    fs::read_to_string,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::client::TmuxClient;

#[test]
fn client_create_session() -> Result<()> {
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string() == "tmux new-session -d -s test -c /tmp")
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string() == "tmux set-option -t test default-shell /bin/zsh")
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string().contains("new-window"))
        .returning(|_| Ok("test_window_id".to_string()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| cmd.to_string().contains("select-layout"))
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux_client = TmuxClient::new(Rc::new(runner));
    let session_name = "test";

    tmux_client.create_session(
        &String::from("test"),
        &String::from("/tmp"),
        &HashMap::new(),
        &Some("/bin/zsh".to_string()),
    )?;
    tmux_client.new_window(session_name, "test", "/tmp")?;
    tmux_client.select_layout(&tmux_target!(session_name, "@1"), "main-horizontal")?;
    Ok(())
}

lazy_static! {
    static ref WIN_NUM: AtomicUsize = AtomicUsize::new(0);
    static ref PANE_NUM: AtomicUsize = AtomicUsize::new(0);
}

#[test]
fn mux_start_session() {
    let path = PathBuf::from_str("./src/common/config/test/valid.yaml").unwrap();

    let session = Session::from_config(&path).unwrap();

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_bool
        .expect_run()
        .withf(|cmd| {
            matches!(
                cmd,
                Type::Basic(_)
                if cmd.to_string() == "tmux has-session -t valid"
            )
        })
        .times(1)
        .returning(|_| Ok(false));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "printenv TMUX"))
        .times(2)
        .returning(|_| Ok("something".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux display-message -p width: #{window_width}\nheight: #{window_height}")
        ).returning(|_| Ok("width: 160\nheight: 90".to_string()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Verbose(_) if vec!["date", "echo Hi"].contains(&cmd.to_string().as_str())),
        )
        .returning(|_| Ok("".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "/tmp/laio-277d3966f692fca8534baf09ce5fc483c928868d776993609681f6d524184281"))
        .returning(|_| Ok("".to_string()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux new-session -d -s valid -c /tmp -e FOO=bar" ))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux set-environment -t valid LAIO_CONFIG ./src/common/config/test/valid.yaml" ))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux show-options -g base-index" ))
        .returning(|_| Ok("base-index 1".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux display-message -t valid -p #I" ))
        .returning(|_| {
                let value = WIN_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("@{}", value))
             }
         );

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux rename-window -t valid:@1 code"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux display-message -t valid:@1 -p #P" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{}", value))
            }
        );

    cmd_unit
        .expect_run()
        .times(3)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-layout -t valid:@1 tiled"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-pane -t valid:@1.%2 -P bg=red,fg=default"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-layout -t valid:@1 tiled"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux split-window -t valid:@1 -c /tmp -P -F #{pane_id}" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{}", value))
            }
        );

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux split-window -t valid:@1 -c /tmp/src -P -F #{pane_id}" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{}", value))
            }
        );

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-layout -t valid:@1 83ed,160x90,0,0[160x45,0,0{53x45,0,0,2,106x45,54,0,3},160x44,0,46,4]"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux new-window -Pd -t valid -n infrastructure -c /tmp/one -F #{window_id}" ))
        .returning(|_| {
                let value = WIN_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("@{}", value))
            }
        );

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux display-message -t valid:@2 -p #P" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{}", value))
            }
        );

    cmd_unit
        .expect_run()
        .times(3)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-layout -t valid:@2 tiled"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux split-window -t valid:@2 -c /tmp/two -P -F #{pane_id}" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{}", value))
            }
        );

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux split-window -t valid:@2 -c /tmp/three -P -F #{pane_id}" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{}", value))
            }
        );

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-layout -t valid:@2 149e,160x90,0,0[160x22,0,0,5,160x45,0,23,6,160x21,0,69,7]"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux bind-key -T prefix M-l display-popup -w 50 -h 16 -E 'laio start --show-picker'"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@1.%1 echo \"hello again\" C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@1.%1 /tmp/laio-46af5b4b2b58c5e6fd4642e48747df751a2c742658faed7ea278b3ed20a9e668 C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux resize-pane -Z -t valid:@1.%4"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@1.%4 echo \"hello again\" C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@1.%1 tmux select-pane -t valid:@1.%1 -T foo  C-m"))
        .times(1)
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@2.%5 echo \"hello again 1\" C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@2.%6 echo \"hello again 2\" C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@2.%7 clear C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@2.%7 echo \"hello again 3\" C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-pane -Z -t valid:@1.%2"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux switch-client -t valid"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.start(
        &session,
        "./src/common/config/test/valid.yaml",
        false,
        false,
    );

    assert!(result.is_ok());
}

#[test]
fn mux_stop_session() -> Result<()> {
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_bool
        .expect_run()
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux has-session -t valid"),
        )
        .times(2)
        .returning(|_| Ok(true));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c [ -n \"$TMUX\" ] && tmux display-message -p '#S' || true"))
        .times(1)
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux show-environment -t valid LAIO_CONFIG"))
        .times(2)
        .returning(|_| Ok("LAIO_CONFIG=./src/common/config/test/valid.yaml".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "date"))
        .times(1)
        .returning(|_| Ok("something".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo Bye"))
        .times(1)
        .returning(|_| Ok("Bye".to_string()));

    cmd_unit
        .expect_run()
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux kill-session -t valid"),
        )
        .times(1)
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.stop(&Some("valid".to_string()), false, false);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn mux_get_session() -> Result<()> {
    let to_yaml = |yaml: String| -> Result<String> {
        let tmp_yaml: Value = serde_yaml::from_str(yaml.as_str()).into_diagnostic()?;
        let string_yaml = serde_yaml::to_string(&tmp_yaml).into_diagnostic()?;
        Ok(string_yaml)
    };
    let cwd = current_dir().unwrap();
    let test_yaml_path = format!("{}/src/common/config/test", cwd.to_string_lossy());
    let valid_yaml =
        to_yaml(read_to_string(format!("{}/to_yaml.yaml", test_yaml_path)).into_diagnostic()?)?;

    let cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux list-windows -F \"#{window_name} #{window_layout}\""))
        .times(1)
        .returning(|_| Ok("code e700,282x67,0,0,21\nmisc 7fa2,282x67,0,0{141x67,0,0[141x22,0,0{47x22,0,0,22,46x22,48,0,23,46x22,95,0,24},141x44,0,23,25],140x67,142,0[140x33,142,0,26,140x15,142,34,27,140x17,142,50,28]}".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux display-message -p \"#S\""))
        .times(1)
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux list-panes -s -F #{pane_id} #{pane_current_path}"))
        .times(2)
        .returning(|_| Ok( "%21 /tmp\n%22 /tmp/one\n%23 /tmp/two\n%24 /tmp/three\n%25 /tmp\n%26 /tmp/four\n%27 /tmp/five\n%28 /tmp/six".to_string()
             .to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux list-panes -s -F #{pane_id} #{pane_pid}"))
        .times(1)
        .returning(|_| Ok("%21 123\n%22 124".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "pgrep -P 123"))
        .times(1)
        .returning(|_| Ok("1234".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "ps -p 1234 -o args="))
        .times(1)
        .returning(|_| Ok("$EDITOR foo.yaml".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "pgrep -P 124"))
        .times(1)
        .returning(|_| Ok("1245".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "ps -p 1245 -o args="))
        .times(1)
        .returning(|_| Ok("foo".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.get_session()?;

    let expected_session_yaml = to_yaml(serde_yaml::to_string(&result).into_diagnostic()?)?;
    assert_eq!(valid_yaml, expected_session_yaml);

    Ok(())
}

#[test]
fn mux_list_sessions() -> Result<()> {
    let cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux ls -F #{session_name}"),
        )
        .times(1)
        .returning(|_| Ok("foo\nbar\nbaz\n".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.list_sessions()?;

    let sessions = result;
    assert_eq!(
        &sessions,
        &vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
    );

    Ok(())
}
