use std::{env::current_dir, fs::read_to_string, path::PathBuf, str::FromStr};

use crate::common::{
    cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
    config::Session,
    muxer::Multiplexer,
};
use miette::{Context, IntoDiagnostic, Result};
use serde_valid::json::Value;

use super::Zellij;

#[test]
fn mux_start_session() -> Result<()> {
    let path = PathBuf::from_str("src/common/config/test/valid.yaml").unwrap();
    let path_str = path.to_string_lossy().into_owned();

    let session = Session::from_config(&path).unwrap();
    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf({
            let path_str = path_str.clone();
            move |cmd| matches!(cmd,
              Type::Forget(content) if
              content.starts_with(&format!("LAIO_CONFIG={} zellij --session valid --new-session-with-layout", path_str)))
        })
        .returning(|_| Ok(()));

    cmd_bool
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij list-sessions --short | grep \"valid\""))
        .returning(|_| Ok(false));

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Verbose(content) if vec!["date", "echo Hi"].contains(&content.as_str())),
        )
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    let _result = zellij.start(&session, &path_str, false, false)?;

    Ok(())
}
#[test]
fn mux_stop_session() -> Result<()> {
    let path = PathBuf::from_str("src/common/config/test/valid.yaml").unwrap();
    let path_str = path.to_string_lossy().into_owned();

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(content) if content == "printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(content) if content == "printenv LAIO_CONFIG || true"),
        )
        .returning({
            let path_str = path_str.clone();
            move |_| Ok(path_str.to_string())
        });

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "printenv ZELLIJ"))
        .returning(|_| Ok("0".to_string()));

    cmd_bool
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij list-sessions --short | grep \"valid\""))
        .returning(|_| Ok(true));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij delete-session \"valid\" --force"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Verbose(content) if vec!["date", "echo Bye"].contains(&content.as_str())))
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    let _result = zellij.stop(&Some("valid".to_string()), false, false)?;

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
    let valid_yaml = to_yaml(
        read_to_string(format!("{}/to_yaml.yaml", test_yaml_path))
            .into_diagnostic()
            .wrap_err(format!("Could not load {}", cwd.to_string_lossy()))?,
    )?;
    let valid_kdl = read_to_string(format!("{}/to_yaml.kdl", test_yaml_path))
        .into_diagnostic()
        .wrap_err(format!("Could not load {}", test_yaml_path))?;

    let cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "printenv ZELLIJ"))
        .returning(|_| Ok("0".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(content) if content == "printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(content) if content == "zellij action dump-layout"))
        .returning(move |_| Ok(valid_kdl.to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);
    let result = zellij.get_session()?;

    let expected_session_yaml = to_yaml(serde_yaml::to_string(&result).into_diagnostic()?)?;
    assert_eq!(valid_yaml, expected_session_yaml);
    Ok(())
}
