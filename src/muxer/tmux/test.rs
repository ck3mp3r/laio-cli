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
use miette::Result;
use serde_valid::yaml::FromYamlStr;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
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
        .withf(|cmd| {
            let temp_dir = std::env::temp_dir();
            cmd.to_string()
                == format!(
                    "tmux new-session -d -s test -c {}",
                    temp_dir.to_string_lossy()
                )
        })
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

    let tmux_client = TmuxClient::new(Arc::new(runner));
    let session_name = "test";

    let temp_dir = std::env::temp_dir();
    let temp_dir_str = temp_dir.to_string_lossy();
    tmux_client.create_session(
        &String::from("test"),
        temp_dir_str.as_ref(),
        &HashMap::new(),
        &Some("/bin/zsh".to_string()),
    )?;
    tmux_client.new_window(session_name, "test", &temp_dir_str)?;
    tmux_client.select_layout(&tmux_target!(session_name, "@1"), "main-horizontal")?;
    Ok(())
}

lazy_static! {
    static ref WIN_NUM: AtomicUsize = AtomicUsize::new(0);
    static ref PANE_NUM: AtomicUsize = AtomicUsize::new(0);
}

#[test]
fn mux_start_session() {
    let temp_dir = std::env::temp_dir();
    let temp_dir_lossy = temp_dir.to_string_lossy();
    let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
    let yaml_str =
        include_str!("../../common/config/test/valid.yaml").replace("/tmp", temp_dir_str);
    let session = Session::from_yaml_str(&yaml_str).unwrap();

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
            |cmd| matches!(cmd, Type::Verbose(_) if ["date", "echo Hi"].contains(&cmd.to_string().as_str())),
        )
        .returning(|_| Ok("".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string().contains( "laio-277d3966f692fca8534baf09ce5fc483c928868d776993609681f6d524184281")))
        .returning(|_| Ok("".to_string()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            let temp_dir = std::env::temp_dir();
            let temp_dir_str = temp_dir.to_string_lossy().trim_end_matches('/').to_string();
            cmd.to_string() == format!("tmux new-session -d -s valid -c {temp_dir_str} -e FOO=bar")
        })
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
                Ok(format!("@{value}"))
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
                Ok(format!("%{value}"))
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
        .withf(|cmd| {
            let temp_dir = std::env::temp_dir();
            let temp_dir_lossy = temp_dir.to_string_lossy();
            let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
            let cmd_str = cmd.to_string();
            cmd_str == format!("tmux split-window -t valid:@1 -c {temp_dir_str} -P -F #{{pane_id}}")
        })
        .returning(|_| {
            let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(format!("%{value}"))
        });

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| {
            let mut temp_dir = std::env::temp_dir();
            temp_dir.push("src");
            let temp_dir_lossy = temp_dir.to_string_lossy();
            let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
            let cmd_str = cmd.to_string();
            cmd_str == format!("tmux split-window -t valid:@1 -c {temp_dir_str} -P -F #{{pane_id}}")
        })
        .returning(|_| {
            let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(format!("%{value}"))
        });

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux select-layout -t valid:@1 83ed,160x90,0,0[160x45,0,0{53x45,0,0,2,106x45,54,0,3},160x44,0,46,4]"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| {
            let mut temp_dir = std::env::temp_dir();
            temp_dir.push("one");
            let temp_dir_lossy = temp_dir.to_string_lossy();
            let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
            let cmd_str = cmd.to_string();
            cmd_str == format!("tmux new-window -Pd -t valid -n infrastructure -c {temp_dir_str} -F #{{window_id}}")
        })
        .returning(|_| {
                let value = WIN_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("@{value}"))
            }
        );

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux display-message -t valid:@2 -p #P" ))
        .returning(|_| {
                let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("%{value}"))
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
        .withf(|cmd| {
            let mut temp_dir = std::env::temp_dir();
            temp_dir.push("two");
            let temp_dir_lossy = temp_dir.to_string_lossy();
            let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
            let cmd_str = cmd.to_string();
            cmd_str == format!("tmux split-window -t valid:@2 -c {temp_dir_str} -P -F #{{pane_id}}")
        })
        .returning(|_| {
            let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(format!("%{value}"))
        });

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| {
            let mut temp_dir = std::env::temp_dir();
            temp_dir.push("three");
            let temp_dir_lossy = temp_dir.to_string_lossy();
            let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
            let cmd_str = cmd.to_string();
            cmd_str == format!("tmux split-window -t valid:@2 -c {temp_dir_str} -P -F #{{pane_id}}")
        })
        .returning(|_| {
            let value = PANE_NUM.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(format!("%{value}"))
        });

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

    // Mock pane readiness checks (capture-pane for stability check)
    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string().contains("capture-pane") && cmd.to_string().contains("-p")))
        .returning(|_| Ok("ready".to_string()));

    // Mock PID checks for command completion
    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string().contains("display-message") && cmd.to_string().contains("pane_pid")))
        .returning(|_| Ok("12345".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string().starts_with("pgrep -P")))
        .returning(|_| Err(miette::miette!("No child processes")));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux send-keys -t valid:@1.%1 echo \"hello again\" C-m"))
        .returning(|_| Ok(()));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            let mut path = std::env::temp_dir();
            path.push("laio-46af5b4b2b58c5e6fd4642e48747df751a2c742658faed7ea278b3ed20a9e668");
            matches!(cmd, Type::Basic(_) if cmd.to_string() == format!("tmux send-keys -t valid:@1.%1 {} C-m", path.to_string_lossy()))
    })
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

    if let Err(e) = &result {
        eprintln!("Test failure: {e:?}");
    }
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

    let result = tmux.stop(&Some("valid".to_string()), false, false, false);
    eprintln!("{:?}", result);

    assert!(result.is_ok());
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
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "tmux ls -F #{session_name}|#{session_attached}"),
        )
        .times(1)
        .returning(|_| Ok("foo|1\nbar|0\nbaz|1\n".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let tmux = Tmux::new_with_runner(runner);

    let result = tmux.list_sessions()?;

    let sessions = result;
    assert_eq!(sessions.len(), 3);
    assert_eq!(sessions[0].name, "foo");
    assert_eq!(sessions[0].status, "●");
    assert_eq!(sessions[1].name, "bar");
    assert_eq!(sessions[1].status, "○");
    assert_eq!(sessions[2].name, "baz");
    assert_eq!(sessions[2].status, "●");

    Ok(())
}
