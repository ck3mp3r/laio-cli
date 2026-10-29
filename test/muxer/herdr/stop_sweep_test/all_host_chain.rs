use std::{env, fs};

use miette::{IntoDiagnostic, Result, miette};
use mockall::Sequence;
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;

use super::mux::Herdr;

use super::support_api_mock::*;
use super::support_env::*;
use super::support_models::*;

#[test]
fn stop_all_proceeds_through_hook_failures_and_tears_down() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let failing_file = env::temp_dir().join(format!("laio-hook-fail-{}.yaml", std::process::id()));
    let failing = "---\nname: alpha\npath: /tmp\nshutdown:\n  - command: ls\n    args:\n      - /nonexistent-laio-hook\nwindows:\n  - name: w\n    panes:\n      - flex: 1\n";
    fs::write(&failing_file, failing).into_diagnostic()?;
    let failing_path = failing_file.to_string_lossy().to_string();

    let passing_file = env::temp_dir().join(format!("laio-hook-pass-{}.yaml", std::process::id()));
    let passing = "---\nname: beta\npath: /tmp\nshutdown:\n  - command: echo\n    args:\n      - ok\nwindows:\n  - name: w\n    panes:\n      - flex: 1\n";
    fs::write(&passing_file, passing).into_diagnostic()?;
    let passing_path = passing_file.to_string_lossy().to_string();

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w1", "alpha", laio_tokens()),
                workspace_info_with("w2", "beta", laio_tokens()),
            ])
        });
    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, workspace_id| workspace_id == "w1")
        .returning({
            let tokens = config_tokens(&failing_path);
            move |_, _| Ok(workspace_info_with("w1", "alpha", tokens.clone()))
        });
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "ls /nonexistent-laio-hook"),
        )
        .returning(|_| Err(miette!("hook failed")));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, workspace_id| workspace_id == "w2")
        .returning({
            let tokens = config_tokens(&passing_path);
            move |_, _| Ok(workspace_info_with("w2", "beta", tokens.clone()))
        });
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo ok"))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w2")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'n=0; until herdr session stop laio >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, true, false);
    restore_env("HERDR_ENV", previous_env);
    let _ = fs::remove_file(&failing_file);
    let _ = fs::remove_file(&passing_file);

    let error = result
        .err()
        .ok_or_else(|| miette!("a failing hook should surface an error"))?;

    assert!(
        format!("{error:?}").contains("nonexistent-laio-hook"),
        "report should carry the hook failure: {error:?}"
    );
    Ok(())
}

#[test]
fn stop_all_closes_the_host_workspace_last_after_spawning_the_teardown_chain() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-host");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w-other", "other", laio_tokens()),
                workspace_info_with("w-host", "host", laio_tokens()),
            ])
        });
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-other")
        .returning(|_, _| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'n=0; until herdr session stop laio >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-host")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    result
}

#[test]
fn stop_all_runs_hooks_for_every_owned_workspace_before_its_close() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-host");

    let other_file = env::temp_dir().join(format!("laio-host-other-{}.yaml", std::process::id()));
    let other = "---\nname: other\npath: /tmp\nshutdown:\n  - command: echo\n    args:\n      - other\nwindows:\n  - name: w\n    panes:\n      - flex: 1\n";
    fs::write(&other_file, other).into_diagnostic()?;
    let other_path = other_file.to_string_lossy().to_string();

    let host_file = env::temp_dir().join(format!("laio-host-host-{}.yaml", std::process::id()));
    let host = "---\nname: host\npath: /tmp\nshutdown:\n  - command: echo\n    args:\n      - host\nwindows:\n  - name: w\n    panes:\n      - flex: 1\n";
    fs::write(&host_file, host).into_diagnostic()?;
    let host_path = host_file.to_string_lossy().to_string();

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w-other", "other", laio_tokens()),
                workspace_info_with("w-host", "host", laio_tokens()),
            ])
        });
    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, workspace_id| workspace_id == "w-other")
        .returning({
            let tokens = config_tokens(&other_path);
            move |_, _| Ok(workspace_info_with("w-other", "other", tokens.clone()))
        });
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo other"))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-other")
        .returning(|_, _| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'n=0; until herdr session stop laio >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));
    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, workspace_id| workspace_id == "w-host")
        .returning({
            let tokens = config_tokens(&host_path);
            move |_, _| Ok(workspace_info_with("w-host", "host", tokens.clone()))
        });
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo host"))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-host")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, true, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    let _ = fs::remove_file(&other_file);
    let _ = fs::remove_file(&host_file);

    result
}
